use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use nano_core::{BranchSchema, BranchSpec, BranchType};
use nano_io::events_chunked;
use nano_io::genpart_survey::{
    examples_text, survey_events, text_summary, GenPartRecord, JsonSurveySummary,
};

const CHUNK_SIZE: usize = 2048;

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let schema = genpart_schema()?;
    let survey = read_survey(
        &options.input,
        &schema,
        options.max_events,
        options.examples,
    )?;

    if let Some(path) = &options.out_json {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = JsonSurveySummary::from(&survey);
        fs::write(path, serde_json::to_string_pretty(&json)? + "\n")?;
    }
    if let Some(path) = &options.out_text {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, text_summary(&survey))?;
    }
    if let Some(path) = &options.out_examples {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, examples_text(&survey, options.examples))?;
    }

    println!("processed_events: {}", survey.processed_events);
    println!("events_with_genpart: {}", survey.events_with_genpart);
    println!("events_with_higgs: {}", survey.events_with_higgs);
    println!("events_with_rho0: {}", survey.events_with_rho0);
    println!("events_with_photon: {}", survey.events_with_photon);
    println!(
        "events_with_pi_plus_pi_minus: {}",
        survey.events_with_pi_plus_pi_minus
    );
    println!(
        "events_with_explicit_h_to_rho_gamma: {}",
        survey.events_with_explicit_h_to_rho_gamma
    );
    println!(
        "events_with_explicit_rho_to_pions: {}",
        survey.events_with_explicit_rho_to_pions
    );
    Ok(())
}

#[derive(Debug)]
struct Options {
    input: PathBuf,
    max_events: Option<usize>,
    out_json: Option<PathBuf>,
    out_text: Option<PathBuf>,
    out_examples: Option<PathBuf>,
    examples: usize,
}

impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut args = std::env::args().skip(1);
        let input = args
            .next()
            .map(PathBuf::from)
            .ok_or("usage: h_rho_gamma_genpart_survey <input.root> [max-events] [--out-json path] [--out-text path] [--out-examples path] [--examples N]")?;
        let mut max_events = None;
        let mut out_json = None;
        let mut out_text = None;
        let mut out_examples = None;
        let mut examples = 3;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--out-json" => {
                    out_json = Some(PathBuf::from(
                        args.next().ok_or("--out-json requires a path")?,
                    ));
                }
                "--out-text" | "--out" => {
                    out_text = Some(PathBuf::from(
                        args.next().ok_or("--out-text requires a path")?,
                    ));
                }
                "--out-examples" => {
                    out_examples = Some(PathBuf::from(
                        args.next().ok_or("--out-examples requires a path")?,
                    ));
                }
                "--examples" => {
                    examples = args
                        .next()
                        .ok_or("--examples requires a value")?
                        .parse::<usize>()?;
                }
                value if max_events.is_none() => {
                    max_events = Some(value.parse::<usize>()?);
                }
                value => {
                    return Err(format!("unexpected argument: {value}").into());
                }
            }
        }

        Ok(Self {
            input,
            max_events,
            out_json,
            out_text,
            out_examples,
            examples,
        })
    }
}

fn genpart_schema() -> Result<BranchSchema, Box<dyn Error>> {
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
    ])?)
}

fn read_survey(
    input: &Path,
    schema: &BranchSchema,
    max_events: Option<usize>,
    examples: usize,
) -> Result<nano_io::genpart_survey::GenPartSurvey, Box<dyn Error>> {
    let events = events_chunked(input, schema, CHUNK_SIZE)?;
    let mut records = Vec::new();
    for event in events {
        if max_events.is_some_and(|limit| records.len() >= limit) {
            break;
        }
        let event = event?;
        let run = event.scalar::<u32>("run")?;
        let lumi = event.scalar::<u32>("luminosityBlock")?;
        let event_number = event.scalar::<u64>("event")?;
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

        let particles = (0..n_gen)
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
            .collect::<Vec<_>>();
        records.push((format!("{run}:{lumi}:{event_number}"), particles));
    }
    Ok(survey_events(records, examples))
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
