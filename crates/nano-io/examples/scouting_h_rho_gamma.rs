use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use nano_core::{BranchSchema, BranchSpec, BranchType};
use nano_io::events_chunked;
use nano_io::scouting_hrhogamma::{
    load_branch_mapping, reconstruct_event, EventInputs, HCand, HToRhoGammaBranchMapping,
    HToRhoGammaCuts,
};
use nano_io::writer::{write_events, OutputBranch};

const ENV_INPUT: &str = "NANO_SCOUTING_HRHOGAMMA_FILE";
const DEFAULT_CONFIG_PATH: &str = "configs/scouting/h_rho_gamma.toml";
const BASELINE_TABLE: &str = "[baseline.zcountinghlt_naive]";
const CHUNK_SIZE: usize = 1024;
const MAX_PRINTED_CANDIDATES: usize = 10;

fn main() -> Result<(), Box<dyn Error>> {
    let Some(options) = Options::parse()? else {
        print_usage();
        return Ok(());
    };

    let (cuts, cut_source, mapping, mapping_source) = load_config(&options)?;
    let schema = scouting_schema(&mapping)?;
    let mut root_writer = options
        .root_path
        .as_deref()
        .map(|_| CandidateRootWriter::default());
    let mut csv_writer = options
        .csv_path
        .as_deref()
        .map(CandidateCsvWriter::create)
        .transpose()?;
    println!("input: {}", options.input.display());
    println!("max_events: {}", display_limit(options.max_events));
    println!("cut_source: {cut_source}");
    println!(
        "candidate_output: {}",
        options
            .csv_path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    println!(
        "root_output: {}",
        options
            .root_path
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    println!("branch_schema: ok");
    println!("branch_catalogue_source: {mapping_source}");
    println!(
        "branch_mapping: {}({}, {}, {}, {}), {}({}, {}, {}, {}, {}, {})",
        mapping.photon.semantic_name,
        mapping.photon.count,
        mapping.photon.pt,
        mapping.photon.eta,
        mapping.photon.phi,
        mapping.charged_candidate.semantic_name,
        mapping.charged_candidate.count,
        mapping.charged_candidate.pt,
        mapping.charged_candidate.eta,
        mapping.charged_candidate.phi,
        mapping.charged_candidate.pdg_id,
        mapping.charged_candidate.mass.as_deref().unwrap_or("none")
    );
    println!(
        "constants: m_pi_charged={:.8} m_rho_target={:.5} m_higgs_reference={:.1}",
        cuts.pion_mass, cuts.rho_mass_target, cuts.higgs_mass_reference
    );
    println!(
        "cuts: photon_pt>={:.1} pi1_pt>={:.1} pi2_pt>={:.1} dr_pipi<{:.2} rho_mass=[{:.1},{:.1}] dr_gamma_rho=[{:.1},{:.1}]",
        cuts.photon_min_pt,
        cuts.pi1_min_pt,
        cuts.pi2_min_pt,
        cuts.max_delta_r_pipi,
        cuts.rho_mass_min,
        cuts.rho_mass_max,
        cuts.min_delta_r_gamma_rho,
        cuts.max_delta_r_gamma_rho
    );

    let report = analyze(
        &options.input,
        &schema,
        &cuts,
        &mapping,
        options.max_events,
        csv_writer.as_mut(),
        root_writer.as_mut(),
    )?;

    if let (Some(mut writer), Some(path)) = (root_writer, options.root_path.as_deref()) {
        writer.save(path)?;
    }
    print_report(&report);
    Ok(())
}

#[derive(Debug)]
struct Options {
    input: PathBuf,
    max_events: Option<usize>,
    config_path: PathBuf,
    config_display: String,
    config_explicit: bool,
    csv_path: Option<PathBuf>,
    root_path: Option<PathBuf>,
}

impl Options {
    fn parse() -> Result<Option<Self>, Box<dyn Error>> {
        let mut positional = Vec::new();
        let mut csv_path = None;
        let mut root_path = None;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => return Ok(None),
                "--csv" => {
                    let value = args.next().ok_or("missing value after --csv")?;
                    csv_path = Some(PathBuf::from(value));
                }
                "--root" => {
                    let value = args.next().ok_or("missing value after --root")?;
                    root_path = Some(PathBuf::from(value));
                }
                _ if arg.starts_with("--") => {
                    return Err(format!("unknown option: {arg}").into());
                }
                _ => positional.push(arg),
            }
        }

        if positional.len() > 3 {
            return Err(
                "usage: scouting_h_rho_gamma <input.root> [max-events] [config.toml] [--csv candidates.csv] [--root candidates.root]".into(),
            );
        }

        let input = if positional.is_empty() {
            match env::var(ENV_INPUT) {
                Ok(value) => PathBuf::from(value),
                Err(_) => return Ok(None),
            }
        } else {
            PathBuf::from(positional.remove(0))
        };

        let mut max_events = None;
        let mut config_path = workspace_default_config_path();
        let mut config_display = DEFAULT_CONFIG_PATH.to_string();
        let mut config_explicit = false;

        match positional.as_slice() {
            [] => {}
            [second] => match second.parse::<usize>() {
                Ok(value) => max_events = Some(value),
                Err(_) => {
                    config_path = PathBuf::from(second);
                    config_display = second.clone();
                    config_explicit = true;
                }
            },
            [second, third] => {
                max_events = Some(
                    second
                        .parse::<usize>()
                        .map_err(|err| format!("invalid max event count: {err}"))?,
                );
                config_path = PathBuf::from(third);
                config_display = third.clone();
                config_explicit = true;
            }
            _ => unreachable!("positional length already checked"),
        }

        Ok(Some(Self {
            input,
            max_events,
            config_path,
            config_display,
            config_explicit,
            csv_path,
            root_path,
        }))
    }
}

fn print_usage() {
    println!("usage: scouting_h_rho_gamma <input.root> [max-events] [config.toml] [--csv candidates.csv] [--root candidates.root]");
    println!("or set {ENV_INPUT}=<input.root>");
    println!("default config: {DEFAULT_CONFIG_PATH}");
}

fn display_limit(limit: Option<usize>) -> String {
    limit
        .map(|value| value.to_string())
        .unwrap_or_else(|| "all".to_string())
}

fn workspace_default_config_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join(DEFAULT_CONFIG_PATH))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH))
}

fn load_config(
    options: &Options,
) -> Result<(HToRhoGammaCuts, String, HToRhoGammaBranchMapping, String), Box<dyn Error>> {
    if options.config_path.exists() {
        let cuts = HToRhoGammaCuts::from_config_path(&options.config_path)?;
        let cut_source = format!("{} {}", options.config_display, BASELINE_TABLE);

        let (mapping_source, mapping) = load_branch_mapping(&options.config_path)?;
        return Ok((cuts, cut_source, mapping, mapping_source));
    }

    if options.config_explicit {
        return Err(format!("config file does not exist: {}", options.config_display).into());
    }

    Ok((
        HToRhoGammaCuts::zcountinghlt_naive(),
        "built-in zcountinghlt_naive fallback".to_string(),
        HToRhoGammaBranchMapping::zcountinghlt_naive(),
        "built-in scouting_run3 fallback".to_string(),
    ))
}

fn scouting_schema(mapping: &HToRhoGammaBranchMapping) -> Result<BranchSchema, Box<dyn Error>> {
    let mut specs = vec![
        BranchSpec::new("run", BranchType::U32),
        BranchSpec::new("luminosityBlock", BranchType::U32),
        BranchSpec::new("event", BranchType::U64),
        BranchSpec::new(&mapping.photon.count, BranchType::I32),
        BranchSpec::new(&mapping.photon.pt, BranchType::VecF32),
        BranchSpec::new(&mapping.photon.eta, BranchType::VecF32),
        BranchSpec::new(&mapping.photon.phi, BranchType::VecF32),
        BranchSpec::new(&mapping.charged_candidate.count, BranchType::I32),
        BranchSpec::new(&mapping.charged_candidate.pt, BranchType::VecF32),
        BranchSpec::new(&mapping.charged_candidate.eta, BranchType::VecF32),
        BranchSpec::new(&mapping.charged_candidate.phi, BranchType::VecF32),
        BranchSpec::new(&mapping.charged_candidate.pdg_id, BranchType::VecI32),
    ];
    if let Some(mass) = &mapping.charged_candidate.mass {
        specs.push(BranchSpec::new(mass, BranchType::VecF32).optional());
    }
    Ok(BranchSchema::new(specs)?)
}

fn analyze(
    input: &Path,
    schema: &BranchSchema,
    cuts: &HToRhoGammaCuts,
    mapping: &HToRhoGammaBranchMapping,
    max_events: Option<usize>,
    mut csv_writer: Option<&mut CandidateCsvWriter>,
    mut root_writer: Option<&mut CandidateRootWriter>,
) -> Result<Report, Box<dyn Error>> {
    let mut report = Report::default();
    let events = events_chunked(input, schema, CHUNK_SIZE)?;

    for event in events {
        if max_events.is_some_and(|limit| report.cutflow.all_events >= limit) {
            break;
        }
        let event = event?;
        report.cutflow.all_events += 1;

        let run = event.scalar::<u32>("run")?;
        let luminosity_block = event.scalar::<u32>("luminosityBlock")?;
        let event_number = event.scalar::<u64>("event")?;

        let n_photon = nonnegative_count(
            event.scalar::<i32>(&mapping.photon.count)?,
            &mapping.photon.count,
        )?;
        let photon_pt = event.vector_ref::<f32>(&mapping.photon.pt)?;
        let photon_eta = event.vector_ref::<f32>(&mapping.photon.eta)?;
        let photon_phi = event.vector_ref::<f32>(&mapping.photon.phi)?;
        validate_len(&mapping.photon.pt, photon_pt.len(), n_photon)?;
        validate_len(&mapping.photon.eta, photon_eta.len(), n_photon)?;
        validate_len(&mapping.photon.phi, photon_phi.len(), n_photon)?;

        let n_pfcand = nonnegative_count(
            event.scalar::<i32>(&mapping.charged_candidate.count)?,
            &mapping.charged_candidate.count,
        )?;
        let pfcand_pt = event.vector_ref::<f32>(&mapping.charged_candidate.pt)?;
        let pfcand_eta = event.vector_ref::<f32>(&mapping.charged_candidate.eta)?;
        let pfcand_phi = event.vector_ref::<f32>(&mapping.charged_candidate.phi)?;
        let pfcand_pdg_id = event.vector_ref::<i32>(&mapping.charged_candidate.pdg_id)?;
        validate_len(&mapping.charged_candidate.pt, pfcand_pt.len(), n_pfcand)?;
        validate_len(&mapping.charged_candidate.eta, pfcand_eta.len(), n_pfcand)?;
        validate_len(&mapping.charged_candidate.phi, pfcand_phi.len(), n_pfcand)?;
        validate_len(
            &mapping.charged_candidate.pdg_id,
            pfcand_pdg_id.len(),
            n_pfcand,
        )?;

        let pfcand_mass = if let Some(mass_field) = &mapping.charged_candidate.mass {
            if event.has_physical_branch(mass_field) {
                let mass = event.vector_ref::<f32>(mass_field)?;
                validate_len(mass_field, mass.len(), n_pfcand)?;
                Some(mass)
            } else {
                None
            }
        } else {
            None
        };

        let reco = reconstruct_event(
            EventInputs {
                photon_pt,
                photon_eta,
                photon_phi,
                pfcand_pt,
                pfcand_eta,
                pfcand_phi,
                pfcand_pdg_id,
                pfcand_mass,
            },
            cuts,
        );

        report.plus_pions += reco.plus_pion_count;
        report.minus_pions += reco.minus_pion_count;
        report.pion_mass_sum += reco.source_pion_mass_sum;
        report.pion_mass_count += reco.source_pion_mass_count;

        if reco.photon.is_none() {
            continue;
        }
        report.cutflow.leading_photon += 1;

        if reco.pions.len() < 2 {
            continue;
        }
        report.cutflow.two_pions += 1;

        if !reco.has_os_pt_pair {
            continue;
        }
        report.cutflow.os_pt_pair += 1;

        if !reco.has_pipi_delta_r_pair {
            continue;
        }
        report.cutflow.pipi_delta_r += 1;

        if reco.rho.is_none() {
            continue;
        }
        report.cutflow.rho_mass_window += 1;

        let Some(h) = reco.h else {
            continue;
        };
        report.cutflow.gamma_rho_delta_r += 1;
        report.cutflow.h_candidate += 1;

        let gamma_rho_delta_r = reco
            .gamma_rho_delta_r
            .expect("H candidate requires gamma-rho deltaR");
        let candidate = CandidateSummary {
            run,
            luminosity_block,
            event: event_number,
            gamma_rho_delta_r,
            rho_over_gamma_pt: h.rho.pt / h.gamma.pt,
            h,
        };

        if let Some(writer) = csv_writer.as_deref_mut() {
            writer.write_candidate(&candidate)?;
        }
        if let Some(writer) = root_writer.as_deref_mut() {
            writer.write_candidate(&candidate);
        }

        if report.candidates.len() < MAX_PRINTED_CANDIDATES {
            report.candidates.push(candidate);
        }
    }

    if let Some(writer) = csv_writer.as_deref_mut() {
        writer.flush()?;
    }
    Ok(report)
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

fn print_report(report: &Report) {
    println!("processed_events: {}", report.cutflow.all_events);
    println!(
        "pfcand_pions: plus_211={} minus_211={} avg_source_mass={}",
        report.plus_pions,
        report.minus_pions,
        report
            .average_pfcand_pion_mass()
            .map(|value| format!("{value:.6}"))
            .unwrap_or_else(|| "n/a".to_string())
    );
    println!("accepted_candidates: {}", report.cutflow.h_candidate);
    println!("cutflow:");
    println!("  all_events {}", report.cutflow.all_events);
    println!("  leading_photon_pt {}", report.cutflow.leading_photon);
    println!("  at_least_two_base_pions {}", report.cutflow.two_pions);
    println!("  os_pair_with_pion_pt {}", report.cutflow.os_pt_pair);
    println!("  pipi_delta_r {}", report.cutflow.pipi_delta_r);
    println!("  rho_mass_window {}", report.cutflow.rho_mass_window);
    println!("  gamma_rho_delta_r {}", report.cutflow.gamma_rho_delta_r);
    println!("  h_candidate {}", report.cutflow.h_candidate);
    println!("first_candidates:");
    if report.candidates.is_empty() {
        println!("  none");
        return;
    }

    for (index, candidate) in report.candidates.iter().enumerate() {
        let h_mass_from_p4 = candidate.h.p4.mass();
        println!(
            "  #{:02} run={} luminosityBlock={} event={}",
            index + 1,
            candidate.run,
            candidate.luminosity_block,
            candidate.event
        );
        println!(
            "      photon pt={:.3} eta={:.3} phi={:.3} mass={:.3}",
            candidate.h.gamma.pt,
            candidate.h.gamma.eta,
            candidate.h.gamma.phi,
            candidate.h.gamma.mass
        );
        println!(
            "      pi+ pt={:.3} eta={:.3} phi={:.3} mass={:.3}",
            candidate.h.rho.pi_plus.pt,
            candidate.h.rho.pi_plus.eta,
            candidate.h.rho.pi_plus.phi,
            candidate.h.rho.pi_plus.mass
        );
        println!(
            "      pi- pt={:.3} eta={:.3} phi={:.3} mass={:.3}",
            candidate.h.rho.pi_minus.pt,
            candidate.h.rho.pi_minus.eta,
            candidate.h.rho.pi_minus.phi,
            candidate.h.rho.pi_minus.mass
        );
        println!(
            "      rho mass={:.3} pt={:.3} eta={:.3} phi={:.3}",
            candidate.h.rho.mass, candidate.h.rho.pt, candidate.h.rho.eta, candidate.h.rho.phi
        );
        println!(
            "      h mass={:.3} p4_mass={:.3} pt={:.3} eta={:.3} phi={:.3}",
            candidate.h.mass, h_mass_from_p4, candidate.h.pt, candidate.h.eta, candidate.h.phi
        );
        println!(
            "      deltaR(pi,pi)={:.4} deltaR(gamma,rho)={:.4} rho_pt/photon_pt={:.4}",
            candidate.h.rho.pipi_delta_r, candidate.gamma_rho_delta_r, candidate.rho_over_gamma_pt
        );
    }
}

#[derive(Debug, Default)]
struct Report {
    cutflow: Cutflow,
    plus_pions: usize,
    minus_pions: usize,
    pion_mass_sum: f64,
    pion_mass_count: usize,
    candidates: Vec<CandidateSummary>,
}

impl Report {
    fn average_pfcand_pion_mass(&self) -> Option<f64> {
        (self.pion_mass_count > 0).then_some(self.pion_mass_sum / self.pion_mass_count as f64)
    }
}

#[derive(Debug, Default)]
struct Cutflow {
    all_events: usize,
    leading_photon: usize,
    two_pions: usize,
    os_pt_pair: usize,
    pipi_delta_r: usize,
    rho_mass_window: usize,
    gamma_rho_delta_r: usize,
    h_candidate: usize,
}

#[derive(Debug)]
struct CandidateSummary {
    run: u32,
    luminosity_block: u32,
    event: u64,
    h: HCand,
    gamma_rho_delta_r: f64,
    rho_over_gamma_pt: f64,
}

struct CandidateCsvWriter {
    writer: BufWriter<File>,
}

impl CandidateCsvWriter {
    fn create(path: &Path) -> Result<Self, Box<dyn Error>> {
        let file = File::create(path)
            .map_err(|err| format!("failed to create candidate CSV {}: {err}", path.display()))?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, "{}", candidate_csv_header())?;
        Ok(Self { writer })
    }

    fn write_candidate(&mut self, candidate: &CandidateSummary) -> Result<(), Box<dyn Error>> {
        let h = &candidate.h;
        writeln!(
            self.writer,
            "{},{},{},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8}",
            candidate.run,
            candidate.luminosity_block,
            candidate.event,
            h.gamma.pt,
            h.gamma.eta,
            h.gamma.phi,
            h.rho.pi_plus.pt,
            h.rho.pi_plus.eta,
            h.rho.pi_plus.phi,
            h.rho.pi_minus.pt,
            h.rho.pi_minus.eta,
            h.rho.pi_minus.phi,
            h.rho.mass,
            h.rho.pt,
            h.rho.eta,
            h.rho.phi,
            h.mass,
            h.pt,
            h.eta,
            h.phi,
            h.rho.pipi_delta_r,
            candidate.gamma_rho_delta_r,
            candidate.rho_over_gamma_pt
        )?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        self.writer.flush()?;
        Ok(())
    }
}

fn candidate_csv_header() -> &'static str {
    "run,luminosityBlock,event,photon_pt,photon_eta,photon_phi,pi_plus_pt,pi_plus_eta,pi_plus_phi,pi_minus_pt,pi_minus_eta,pi_minus_phi,rho_mass,rho_pt,rho_eta,rho_phi,h_mass,h_pt,h_eta,h_phi,delta_r_pipi,delta_r_gamma_rho,rho_pt_over_photon_pt"
}

#[derive(Debug, Default)]
struct CandidateRootWriter {
    run: Vec<u32>,
    luminosity_block: Vec<u32>,
    event: Vec<u64>,
    photon_pt: Vec<f32>,
    photon_eta: Vec<f32>,
    photon_phi: Vec<f32>,
    pi_plus_pt: Vec<f32>,
    pi_plus_eta: Vec<f32>,
    pi_plus_phi: Vec<f32>,
    pi_minus_pt: Vec<f32>,
    pi_minus_eta: Vec<f32>,
    pi_minus_phi: Vec<f32>,
    rho_mass: Vec<f32>,
    rho_pt: Vec<f32>,
    rho_eta: Vec<f32>,
    rho_phi: Vec<f32>,
    h_mass: Vec<f32>,
    h_pt: Vec<f32>,
    h_eta: Vec<f32>,
    h_phi: Vec<f32>,
    delta_r_pipi: Vec<f32>,
    delta_r_gamma_rho: Vec<f32>,
    rho_pt_over_photon_pt: Vec<f32>,
}

impl CandidateRootWriter {
    fn write_candidate(&mut self, candidate: &CandidateSummary) {
        let h = &candidate.h;
        self.run.push(candidate.run);
        self.luminosity_block.push(candidate.luminosity_block);
        self.event.push(candidate.event);
        self.photon_pt.push(h.gamma.pt as f32);
        self.photon_eta.push(h.gamma.eta as f32);
        self.photon_phi.push(h.gamma.phi as f32);
        self.pi_plus_pt.push(h.rho.pi_plus.pt as f32);
        self.pi_plus_eta.push(h.rho.pi_plus.eta as f32);
        self.pi_plus_phi.push(h.rho.pi_plus.phi as f32);
        self.pi_minus_pt.push(h.rho.pi_minus.pt as f32);
        self.pi_minus_eta.push(h.rho.pi_minus.eta as f32);
        self.pi_minus_phi.push(h.rho.pi_minus.phi as f32);
        self.rho_mass.push(h.rho.mass as f32);
        self.rho_pt.push(h.rho.pt as f32);
        self.rho_eta.push(h.rho.eta as f32);
        self.rho_phi.push(h.rho.phi as f32);
        self.h_mass.push(h.mass as f32);
        self.h_pt.push(h.pt as f32);
        self.h_eta.push(h.eta as f32);
        self.h_phi.push(h.phi as f32);
        self.delta_r_pipi.push(h.rho.pipi_delta_r as f32);
        self.delta_r_gamma_rho
            .push(candidate.gamma_rho_delta_r as f32);
        self.rho_pt_over_photon_pt
            .push(candidate.rho_over_gamma_pt as f32);
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let branches = vec![
            OutputBranch::u32("run", std::mem::take(&mut self.run)),
            OutputBranch::u32(
                "luminosityBlock",
                std::mem::take(&mut self.luminosity_block),
            ),
            OutputBranch::u64("event", std::mem::take(&mut self.event)),
            OutputBranch::f32("photon_pt", std::mem::take(&mut self.photon_pt)),
            OutputBranch::f32("photon_eta", std::mem::take(&mut self.photon_eta)),
            OutputBranch::f32("photon_phi", std::mem::take(&mut self.photon_phi)),
            OutputBranch::f32("pi_plus_pt", std::mem::take(&mut self.pi_plus_pt)),
            OutputBranch::f32("pi_plus_eta", std::mem::take(&mut self.pi_plus_eta)),
            OutputBranch::f32("pi_plus_phi", std::mem::take(&mut self.pi_plus_phi)),
            OutputBranch::f32("pi_minus_pt", std::mem::take(&mut self.pi_minus_pt)),
            OutputBranch::f32("pi_minus_eta", std::mem::take(&mut self.pi_minus_eta)),
            OutputBranch::f32("pi_minus_phi", std::mem::take(&mut self.pi_minus_phi)),
            OutputBranch::f32("rho_mass", std::mem::take(&mut self.rho_mass)),
            OutputBranch::f32("rho_pt", std::mem::take(&mut self.rho_pt)),
            OutputBranch::f32("rho_eta", std::mem::take(&mut self.rho_eta)),
            OutputBranch::f32("rho_phi", std::mem::take(&mut self.rho_phi)),
            OutputBranch::f32("h_mass", std::mem::take(&mut self.h_mass)),
            OutputBranch::f32("h_pt", std::mem::take(&mut self.h_pt)),
            OutputBranch::f32("h_eta", std::mem::take(&mut self.h_eta)),
            OutputBranch::f32("h_phi", std::mem::take(&mut self.h_phi)),
            OutputBranch::f32("delta_r_pipi", std::mem::take(&mut self.delta_r_pipi)),
            OutputBranch::f32(
                "delta_r_gamma_rho",
                std::mem::take(&mut self.delta_r_gamma_rho),
            ),
            OutputBranch::f32(
                "rho_pt_over_photon_pt",
                std::mem::take(&mut self.rho_pt_over_photon_pt),
            ),
        ];
        write_events(path, &branches)?;
        Ok(())
    }
}
