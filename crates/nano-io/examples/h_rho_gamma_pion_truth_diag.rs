use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use nano_core::{BranchSchema, BranchSpec, BranchType};
use nano_io::events_chunked;
use nano_io::genpart_survey::GenPartRecord;
use nano_io::pion_truth_diagnosis::{recommendation, PionTruthDiagnosisSummary, RecoCandidate};

const CHUNK_SIZE: usize = 2048;

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let candidates = read_candidates(&options.candidate_csv, options.max_candidates)?;
    let schema = schema()?;
    let mut summary = diagnose_file(&options.input, &schema, &candidates, options.max_events)?;

    if let Some(path) = &options.out_json {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut value = serde_json::to_value(&summary)?;
        if let serde_json::Value::Object(ref mut object) = value {
            object.insert(
                "recommendation".to_string(),
                serde_json::Value::String(recommendation(&summary).to_string()),
            );
        }
        fs::write(path, serde_json::to_string_pretty(&value)? + "\n")?;
    }

    if let Some(path) = &options.out_text {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, text_summary(&summary))?;
    }

    println!("candidate_rows: {}", summary.candidate_rows);
    println!("diagnosed_candidates: {}", summary.diagnosed_candidates);
    println!(
        "packed_genpart_branches_present: {}",
        summary.packed_genpart_branches_present
    );
    println!("recommendation: {}", recommendation(&summary));

    // Keep the value mutable above for JSON insertion without cloning the large
    // candidate diagnostics vector.
    summary.candidate_diagnostics.clear();
    Ok(())
}

#[derive(Debug)]
struct Options {
    input: PathBuf,
    max_events: Option<usize>,
    candidate_csv: PathBuf,
    out_json: Option<PathBuf>,
    out_text: Option<PathBuf>,
    max_candidates: Option<usize>,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut args = std::env::args().skip(1);
        let input = args
            .next()
            .map(PathBuf::from)
            .ok_or("usage: h_rho_gamma_pion_truth_diag <input.root> [max-events] --candidate-csv candidates.csv [--out-json path] [--out-text path] [--max-candidates N]")?;
        let mut max_events = None;
        let mut candidate_csv = None;
        let mut out_json = None;
        let mut out_text = None;
        let mut max_candidates = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--candidate-csv" => {
                    candidate_csv = Some(PathBuf::from(
                        args.next().ok_or("--candidate-csv requires a path")?,
                    ));
                }
                "--out-json" => {
                    out_json = Some(PathBuf::from(
                        args.next().ok_or("--out-json requires a path")?,
                    ));
                }
                "--out-text" => {
                    out_text = Some(PathBuf::from(
                        args.next().ok_or("--out-text requires a path")?,
                    ));
                }
                "--max-candidates" => {
                    max_candidates = Some(
                        args.next()
                            .ok_or("--max-candidates requires a value")?
                            .parse::<usize>()?,
                    );
                }
                value if max_events.is_none() => {
                    max_events = Some(value.parse::<usize>()?);
                }
                value => return Err(format!("unexpected argument: {value}").into()),
            }
        }

        Ok(Self {
            input,
            max_events,
            candidate_csv: candidate_csv.ok_or("--candidate-csv is required")?,
            out_json,
            out_text,
            max_candidates,
        })
    }
}

fn schema() -> Result<BranchSchema, Box<dyn Error>> {
    Ok(BranchSchema::new(vec![
        BranchSpec::new("run", BranchType::U32),
        BranchSpec::new("luminosityBlock", BranchType::U32),
        BranchSpec::new("event", BranchType::U64),
        BranchSpec::new("nGenPart", BranchType::I32),
        BranchSpec::new("GenPart_pdgId", BranchType::VecI32),
        BranchSpec::new("GenPart_genPartIdxMother", BranchType::VecI16),
        BranchSpec::new("GenPart_status", BranchType::VecI32),
        BranchSpec::new("GenPart_statusFlags", BranchType::VecU16),
        BranchSpec::new("GenPart_pt", BranchType::VecF32),
        BranchSpec::new("GenPart_eta", BranchType::VecF32),
        BranchSpec::new("GenPart_phi", BranchType::VecF32),
        BranchSpec::new("GenPart_mass", BranchType::VecF32),
        BranchSpec::new("nPackedGenPart", BranchType::I32).optional(),
        BranchSpec::new("PackedGenPart_pt", BranchType::VecF32).optional(),
        BranchSpec::new("PackedGenPart_eta", BranchType::VecF32).optional(),
        BranchSpec::new("PackedGenPart_phi", BranchType::VecF32).optional(),
        BranchSpec::new("PackedGenPart_mass", BranchType::VecF32).optional(),
        BranchSpec::new("PackedGenPart_pdgId", BranchType::VecI32).optional(),
        BranchSpec::new("PackedGenPart_status", BranchType::VecI32).optional(),
        BranchSpec::new("PackedGenPart_statusFlags", BranchType::VecU16).optional(),
    ])?)
}

type EventKey = (u32, u32, u64);

fn diagnose_file(
    input: &Path,
    schema: &BranchSchema,
    candidates: &BTreeMap<EventKey, Vec<RecoCandidate>>,
    max_events: Option<usize>,
) -> Result<PionTruthDiagnosisSummary, Box<dyn Error>> {
    let candidate_rows = candidates
        .values()
        .map(|items| items.len() as u64)
        .sum::<u64>();
    let mut summary = PionTruthDiagnosisSummary::new(candidate_rows);
    if candidate_rows == 0 {
        return Ok(summary);
    }

    let events = events_chunked(input, schema, CHUNK_SIZE)?;
    let mut processed = 0usize;
    for event in events {
        if max_events.is_some_and(|limit| processed >= limit) {
            break;
        }
        let event = event?;
        processed += 1;
        let run = event.scalar::<u32>("run")?;
        let lumi = event.scalar::<u32>("luminosityBlock")?;
        let event_number = event.scalar::<u64>("event")?;
        let Some(event_candidates) = candidates.get(&(run, lumi, event_number)) else {
            continue;
        };

        summary.packed_genpart_branches_present |= event.has_physical_branch("nPackedGenPart")
            || event.has_physical_branch("PackedGenPart_pt")
            || event.has_physical_branch("PackedGenPart_pdgId");
        let particles = read_particles(&event)?;
        summary.add_event_candidates(event_candidates, &particles);

        if summary.diagnosed_candidates >= candidate_rows {
            break;
        }
    }
    Ok(summary)
}

fn read_particles(event: &nano_core::Event) -> Result<Vec<GenPartRecord>, Box<dyn Error>> {
    let n_gen = nonnegative_count(event.scalar::<i32>("nGenPart")?, "nGenPart")?;
    let pdg_id = event.vector_ref::<i32>("GenPart_pdgId")?;
    let mother = event.vector_ref::<i16>("GenPart_genPartIdxMother")?;
    let status = event.vector_ref::<i32>("GenPart_status")?;
    let status_flags = event.vector_ref::<u16>("GenPart_statusFlags")?;
    let pt = event.vector_ref::<f32>("GenPart_pt")?;
    let eta = event.vector_ref::<f32>("GenPart_eta")?;
    let phi = event.vector_ref::<f32>("GenPart_phi")?;
    let mass = event.vector_ref::<f32>("GenPart_mass")?;
    validate_len("GenPart_pdgId", pdg_id.len(), n_gen)?;
    validate_len("GenPart_genPartIdxMother", mother.len(), n_gen)?;
    validate_len("GenPart_status", status.len(), n_gen)?;
    validate_len("GenPart_statusFlags", status_flags.len(), n_gen)?;
    validate_len("GenPart_pt", pt.len(), n_gen)?;
    validate_len("GenPart_eta", eta.len(), n_gen)?;
    validate_len("GenPart_phi", phi.len(), n_gen)?;
    validate_len("GenPart_mass", mass.len(), n_gen)?;

    Ok((0..n_gen)
        .map(|index| GenPartRecord {
            pdg_id: pdg_id[index],
            mother: usize::try_from(mother[index]).ok(),
            status: status[index],
            status_flags: status_flags[index],
            pt: f64::from(pt[index]),
            eta: f64::from(eta[index]),
            phi: f64::from(phi[index]),
            mass: f64::from(mass[index]),
        })
        .collect())
}

fn read_candidates(
    path: &Path,
    max_candidates: Option<usize>,
) -> Result<BTreeMap<EventKey, Vec<RecoCandidate>>, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    let mut lines = contents.lines();
    let header = lines.next().ok_or("candidate CSV is empty")?;
    let fields = header.split(',').collect::<Vec<_>>();
    let mut columns = BTreeMap::new();
    for (index, name) in fields.iter().enumerate() {
        columns.insert(*name, index);
    }

    let mut candidates: BTreeMap<EventKey, Vec<RecoCandidate>> = BTreeMap::new();
    for (row_index, line) in lines.enumerate() {
        if max_candidates.is_some_and(|limit| row_index >= limit) {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let values = line.split(',').collect::<Vec<_>>();
        let candidate = RecoCandidate {
            run: parse_value("run", &values, &columns)?,
            luminosity_block: parse_value("luminosityBlock", &values, &columns)?,
            event: parse_value("event", &values, &columns)?,
            pi_plus_pt: parse_value("pi_plus_pt", &values, &columns)?,
            pi_plus_eta: parse_value("pi_plus_eta", &values, &columns)?,
            pi_plus_phi: parse_value("pi_plus_phi", &values, &columns)?,
            pi_minus_pt: parse_value("pi_minus_pt", &values, &columns)?,
            pi_minus_eta: parse_value("pi_minus_eta", &values, &columns)?,
            pi_minus_phi: parse_value("pi_minus_phi", &values, &columns)?,
            h_mass: parse_value("h_mass", &values, &columns)?,
            rho_mass: parse_value("rho_mass", &values, &columns)?,
            rho_pt_over_photon_pt: parse_value("rho_pt_over_photon_pt", &values, &columns)?,
            photon_matched: parse_optional_bool("truth_photon_anchor_available", &values, &columns)
                .unwrap_or(false),
        };
        candidates
            .entry((candidate.run, candidate.luminosity_block, candidate.event))
            .or_default()
            .push(candidate);
    }
    Ok(candidates)
}

fn parse_value<T: std::str::FromStr>(
    name: &str,
    values: &[&str],
    columns: &BTreeMap<&str, usize>,
) -> Result<T, Box<dyn Error>>
where
    T::Err: Error + 'static,
{
    let index = *columns
        .get(name)
        .ok_or_else(|| format!("candidate CSV missing column {name}"))?;
    values
        .get(index)
        .ok_or_else(|| format!("candidate CSV row missing column {name}"))?
        .parse::<T>()
        .map_err(|err| err.into())
}

fn parse_optional_bool(
    name: &str,
    values: &[&str],
    columns: &BTreeMap<&str, usize>,
) -> Option<bool> {
    let index = *columns.get(name)?;
    let raw = *values.get(index)?;
    Some(matches!(raw, "1" | "true" | "True" | "TRUE"))
}

fn nonnegative_count(value: i32, name: &str) -> Result<usize, Box<dyn Error>> {
    usize::try_from(value).map_err(|_| format!("{name} is negative: {value}").into())
}

fn validate_len(name: &str, actual: usize, expected: usize) -> Result<(), Box<dyn Error>> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{name} length {actual} does not match count {expected}").into())
    }
}

fn text_summary(summary: &PionTruthDiagnosisSummary) -> String {
    let mut lines = vec![
        "HToRhoGamma charged-pion truth-proxy diagnosis".to_string(),
        format!("candidate_rows: {}", summary.candidate_rows),
        format!("diagnosed_candidates: {}", summary.diagnosed_candidates),
        format!(
            "packed_genpart_branches_present: {}",
            summary.packed_genpart_branches_present
        ),
        format!("recommendation: {}", recommendation(summary)),
        "".to_string(),
        "category_counts:".to_string(),
    ];
    for (category, count) in &summary.category_counts {
        lines.push(format!("  {category}: {count}"));
    }
    lines.push("".to_string());
    lines.push("pool_event_counts:".to_string());
    for (pool, count) in &summary.pool_event_counts {
        lines.push(format!("  {pool}: {count}"));
    }
    lines.join("\n") + "\n"
}
