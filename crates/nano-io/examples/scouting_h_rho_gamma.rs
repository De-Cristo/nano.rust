use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use nano_core::{BranchSchema, BranchSpec, BranchType};
use nano_io::events_chunked;
use nano_io::scouting_hrhogamma::{
    load_branch_mapping, match_reco_to_truth_proxy, reconstruct_event, EventInputs, GenParticle,
    HCand, HToRhoGammaBranchMapping, HToRhoGammaCuts, TruthMatchResult, TruthTopology,
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
    let schema = scouting_schema(&mapping, options.truth)?;
    let mut root_writer = options
        .root_path
        .as_deref()
        .map(|_| CandidateRootWriter::new(options.truth));
    let mut csv_writer = options
        .csv_path
        .as_deref()
        .map(|path| CandidateCsvWriter::create(path, options.truth))
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
    println!("truth: {}", options.truth);
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
        options.truth,
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
    truth: bool,
}

impl Options {
    fn parse() -> Result<Option<Self>, Box<dyn Error>> {
        let mut positional = Vec::new();
        let mut csv_path = None;
        let mut root_path = None;
        let mut truth = false;

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
                "--truth" => {
                    truth = true;
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
            truth,
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

fn scouting_schema(
    mapping: &HToRhoGammaBranchMapping,
    truth: bool,
) -> Result<BranchSchema, Box<dyn Error>> {
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
    if truth {
        specs.extend([
            BranchSpec::new("nGenPart", BranchType::I32).optional(),
            BranchSpec::new("GenPart_pdgId", BranchType::VecI32).optional(),
            BranchSpec::new("GenPart_genPartIdxMother", BranchType::VecI16).optional(),
            BranchSpec::new("GenPart_status", BranchType::VecI32).optional(),
            BranchSpec::new("GenPart_statusFlags", BranchType::VecU16).optional(),
            BranchSpec::new("GenPart_pt", BranchType::VecF32).optional(),
            BranchSpec::new("GenPart_eta", BranchType::VecF32).optional(),
            BranchSpec::new("GenPart_phi", BranchType::VecF32).optional(),
            BranchSpec::new("GenPart_mass", BranchType::VecF32).optional(),
        ]);
    }
    Ok(BranchSchema::new(specs)?)
}

fn analyze(
    input: &Path,
    schema: &BranchSchema,
    cuts: &HToRhoGammaCuts,
    mapping: &HToRhoGammaBranchMapping,
    max_events: Option<usize>,
    truth_enabled: bool,
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
        let truth = if truth_enabled {
            let particles = read_gen_particles(&event)?;
            if particles.is_some() {
                report.truth_available_events += 1;
            }
            Some(match_reco_to_truth_proxy(&h, particles.as_deref()))
        } else {
            None
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
            truth,
        };

        if let Some(truth) = candidate.truth {
            match truth.truth_topology {
                TruthTopology::ExplicitRho => report.truth_explicit_rho += 1,
                TruthTopology::FallbackNoExplicitRho => report.truth_fallback += 1,
                TruthTopology::NotFound => report.truth_not_found += 1,
                TruthTopology::NotAvailable => report.truth_not_available += 1,
            }
            if truth.truth_matched {
                report.truth_matched += 1;
            }
            if truth.truth_proxy_matched_dr_0p1 {
                report.truth_proxy_matched_0p1 += 1;
            }
            if truth.truth_proxy_matched_dr_0p2 {
                report.truth_proxy_matched_0p2 += 1;
            }
            if truth.truth_proxy_matched_dr_0p3 {
                report.truth_proxy_matched_0p3 += 1;
            }
        }

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

fn read_gen_particles(
    event: &nano_core::Event,
) -> Result<Option<Vec<GenParticle>>, Box<dyn Error>> {
    const REQUIRED: [&str; 9] = [
        "nGenPart",
        "GenPart_pdgId",
        "GenPart_genPartIdxMother",
        "GenPart_status",
        "GenPart_statusFlags",
        "GenPart_pt",
        "GenPart_eta",
        "GenPart_phi",
        "GenPart_mass",
    ];
    if !REQUIRED
        .iter()
        .all(|branch| event.has_physical_branch(branch))
    {
        return Ok(None);
    }
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
        .map(|index| {
            let mother_index = usize::try_from(mother[index]).ok();
            GenParticle::with_status(
                pdg_id[index],
                mother_index,
                f64::from(pt[index]),
                f64::from(eta[index]),
                f64::from(phi[index]),
                f64::from(mass[index]),
                status[index],
                status_flags[index],
            )
        })
        .collect::<Vec<_>>();
    Ok(Some(particles))
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
    println!(
        "truth_summary: available_events={} matched_candidates={} explicit_rho={} fallback_no_explicit_rho={} not_found={} not_available={}",
        report.truth_available_events,
        report.truth_matched,
        report.truth_explicit_rho,
        report.truth_fallback,
        report.truth_not_found,
        report.truth_not_available
    );
    println!(
        "truth_proxy_summary: matched_dr_0p1={} matched_dr_0p2={} matched_dr_0p3={}",
        report.truth_proxy_matched_0p1,
        report.truth_proxy_matched_0p2,
        report.truth_proxy_matched_0p3
    );
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
    truth_available_events: usize,
    truth_matched: usize,
    truth_explicit_rho: usize,
    truth_fallback: usize,
    truth_not_found: usize,
    truth_not_available: usize,
    truth_proxy_matched_0p1: usize,
    truth_proxy_matched_0p2: usize,
    truth_proxy_matched_0p3: usize,
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
    truth: Option<TruthMatchResult>,
}

struct CandidateCsvWriter {
    writer: BufWriter<File>,
    truth: bool,
}

impl CandidateCsvWriter {
    fn create(path: &Path, truth: bool) -> Result<Self, Box<dyn Error>> {
        let file = File::create(path)
            .map_err(|err| format!("failed to create candidate CSV {}: {err}", path.display()))?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, "{}", candidate_csv_header(truth))?;
        Ok(Self { writer, truth })
    }

    fn write_candidate(&mut self, candidate: &CandidateSummary) -> Result<(), Box<dyn Error>> {
        let h = &candidate.h;
        write!(
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
        if self.truth {
            let truth = candidate
                .truth
                .unwrap_or_else(TruthMatchResult::not_available);
            let fields = vec![
                truth.truth_strategy.as_str().to_string(),
                (truth.truth_available as u8).to_string(),
                (truth.truth_proxy_matched as u8).to_string(),
                (truth.truth_proxy_matched_dr_0p1 as u8).to_string(),
                (truth.truth_proxy_matched_dr_0p2 as u8).to_string(),
                (truth.truth_proxy_matched_dr_0p3 as u8).to_string(),
                (truth.truth_photon_anchor_available as u8).to_string(),
                (truth.gen_photon_from_higgs as u8).to_string(),
                opt(truth.gen_photon.map(|p| p.pt)),
                opt(truth.gen_photon.map(|p| p.eta)),
                opt(truth.gen_photon.map(|p| p.phi)),
                opt(truth.gen_photon.map(|p| p.mass)),
                (truth.nearest_gen_pi_plus_available as u8).to_string(),
                opt(truth.gen_pi_plus.map(|p| p.pt)),
                opt(truth.gen_pi_plus.map(|p| p.eta)),
                opt(truth.gen_pi_plus.map(|p| p.phi)),
                opt(truth.gen_pi_plus.map(|p| p.mass)),
                (truth.nearest_gen_pi_minus_available as u8).to_string(),
                opt(truth.gen_pi_minus.map(|p| p.pt)),
                opt(truth.gen_pi_minus.map(|p| p.eta)),
                opt(truth.gen_pi_minus.map(|p| p.phi)),
                opt(truth.gen_pi_minus.map(|p| p.mass)),
                opt(truth.gen_rho_proxy.map(|p| p.pt)),
                opt(truth.gen_rho_proxy.map(|p| p.eta)),
                opt(truth.gen_rho_proxy.map(|p| p.phi)),
                opt(truth.gen_rho_proxy.map(|p| p.mass)),
                opt(truth.gen_h_proxy.map(|p| p.pt)),
                opt(truth.gen_h_proxy.map(|p| p.eta)),
                opt(truth.gen_h_proxy.map(|p| p.phi)),
                opt(truth.gen_h_proxy.map(|p| p.mass)),
                opt(truth.delta_r_reco_photon_gen_photon),
                opt(truth.delta_r_reco_pi_plus_gen_pi_plus),
                opt(truth.delta_r_reco_pi_minus_gen_pi_minus),
                opt(truth.delta_r_reco_rho_gen_rho_proxy),
                opt(truth.delta_r_reco_h_gen_h_proxy),
                opt(truth.reco_h_mass_minus_gen_h_proxy_mass),
                opt(truth.reco_rho_mass_minus_gen_rho_proxy_mass),
                opt(truth.reco_photon_pt_over_gen_photon_pt),
                opt(truth.reco_pi_plus_pt_over_gen_pi_plus_pt),
                opt(truth.reco_pi_minus_pt_over_gen_pi_minus_pt),
                opt(truth.reco_rho_pt_over_gen_rho_proxy_pt),
                opt(truth.reco_h_pt_over_gen_h_proxy_pt),
            ];
            write!(self.writer, ",{}", fields.join(","))?;
        }
        writeln!(self.writer)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        self.writer.flush()?;
        Ok(())
    }
}

fn candidate_csv_header(truth: bool) -> String {
    let base = "run,luminosityBlock,event,photon_pt,photon_eta,photon_phi,pi_plus_pt,pi_plus_eta,pi_plus_phi,pi_minus_pt,pi_minus_eta,pi_minus_phi,rho_mass,rho_pt,rho_eta,rho_phi,h_mass,h_pt,h_eta,h_phi,delta_r_pipi,delta_r_gamma_rho,rho_pt_over_photon_pt";
    if truth {
        format!("{base},truth_strategy,truth_available,truth_proxy_matched,truth_proxy_matched_dr_0p1,truth_proxy_matched_dr_0p2,truth_proxy_matched_dr_0p3,truth_photon_anchor_available,gen_photon_from_higgs,gen_photon_pt,gen_photon_eta,gen_photon_phi,gen_photon_mass,nearest_gen_pi_plus_available,gen_pi_plus_pt,gen_pi_plus_eta,gen_pi_plus_phi,gen_pi_plus_mass,nearest_gen_pi_minus_available,gen_pi_minus_pt,gen_pi_minus_eta,gen_pi_minus_phi,gen_pi_minus_mass,gen_rho_proxy_pt,gen_rho_proxy_eta,gen_rho_proxy_phi,gen_rho_proxy_mass,gen_h_proxy_pt,gen_h_proxy_eta,gen_h_proxy_phi,gen_h_proxy_mass,delta_r_reco_photon_gen_photon,delta_r_reco_pi_plus_gen_pi_plus,delta_r_reco_pi_minus_gen_pi_minus,delta_r_reco_rho_gen_rho_proxy,delta_r_reco_h_gen_h_proxy,reco_h_mass_minus_gen_h_proxy_mass,reco_rho_mass_minus_gen_rho_proxy_mass,reco_photon_pt_over_gen_photon_pt,reco_pi_plus_pt_over_gen_pi_plus_pt,reco_pi_minus_pt_over_gen_pi_minus_pt,reco_rho_pt_over_gen_rho_proxy_pt,reco_h_pt_over_gen_h_proxy_pt")
    } else {
        base.to_string()
    }
}

fn opt(value: Option<f64>) -> String {
    value.map(|value| format!("{value:.8}")).unwrap_or_default()
}

#[derive(Debug, Default)]
struct CandidateRootWriter {
    truth_enabled: bool,
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
    truth_available: Vec<bool>,
    truth_proxy_matched: Vec<bool>,
    truth_proxy_matched_dr_0p1: Vec<bool>,
    truth_proxy_matched_dr_0p2: Vec<bool>,
    truth_proxy_matched_dr_0p3: Vec<bool>,
    truth_strategy_code: Vec<i32>,
    truth_photon_anchor_available: Vec<bool>,
    gen_photon_from_higgs: Vec<bool>,
    nearest_gen_pi_plus_available: Vec<bool>,
    nearest_gen_pi_minus_available: Vec<bool>,
    truth_matched: Vec<bool>,
    truth_topology_code: Vec<i32>,
    gen_h_pt: Vec<f32>,
    gen_h_eta: Vec<f32>,
    gen_h_phi: Vec<f32>,
    gen_h_mass: Vec<f32>,
    gen_rho_pt: Vec<f32>,
    gen_rho_eta: Vec<f32>,
    gen_rho_phi: Vec<f32>,
    gen_rho_mass: Vec<f32>,
    gen_photon_pt: Vec<f32>,
    gen_photon_eta: Vec<f32>,
    gen_photon_phi: Vec<f32>,
    gen_photon_mass: Vec<f32>,
    gen_pi_plus_pt: Vec<f32>,
    gen_pi_plus_eta: Vec<f32>,
    gen_pi_plus_phi: Vec<f32>,
    gen_pi_plus_mass: Vec<f32>,
    gen_pi_minus_pt: Vec<f32>,
    gen_pi_minus_eta: Vec<f32>,
    gen_pi_minus_phi: Vec<f32>,
    gen_pi_minus_mass: Vec<f32>,
    gen_rho_proxy_pt: Vec<f32>,
    gen_rho_proxy_eta: Vec<f32>,
    gen_rho_proxy_phi: Vec<f32>,
    gen_rho_proxy_mass: Vec<f32>,
    gen_h_proxy_pt: Vec<f32>,
    gen_h_proxy_eta: Vec<f32>,
    gen_h_proxy_phi: Vec<f32>,
    gen_h_proxy_mass: Vec<f32>,
    delta_r_reco_photon_gen_photon: Vec<f32>,
    delta_r_reco_pi_plus_gen_pi_plus: Vec<f32>,
    delta_r_reco_pi_minus_gen_pi_minus: Vec<f32>,
    delta_r_reco_rho_gen_rho: Vec<f32>,
    delta_r_reco_rho_gen_rho_proxy: Vec<f32>,
    delta_r_reco_h_gen_h_proxy: Vec<f32>,
    reco_h_mass_minus_gen_h_mass: Vec<f32>,
    reco_rho_mass_minus_gen_rho_mass: Vec<f32>,
    reco_h_mass_minus_gen_h_proxy_mass: Vec<f32>,
    reco_rho_mass_minus_gen_rho_proxy_mass: Vec<f32>,
    reco_photon_pt_over_gen_photon_pt: Vec<f32>,
    reco_rho_pt_over_gen_rho_pt: Vec<f32>,
    reco_pi_plus_pt_over_gen_pi_plus_pt: Vec<f32>,
    reco_pi_minus_pt_over_gen_pi_minus_pt: Vec<f32>,
    reco_rho_pt_over_gen_rho_proxy_pt: Vec<f32>,
    reco_h_pt_over_gen_h_proxy_pt: Vec<f32>,
}

impl CandidateRootWriter {
    fn new(truth_enabled: bool) -> Self {
        Self {
            truth_enabled,
            ..Self::default()
        }
    }

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
        if self.truth_enabled {
            let truth = candidate
                .truth
                .unwrap_or_else(TruthMatchResult::not_available);
            self.truth_available.push(truth.truth_available);
            self.truth_proxy_matched.push(truth.truth_proxy_matched);
            self.truth_proxy_matched_dr_0p1
                .push(truth.truth_proxy_matched_dr_0p1);
            self.truth_proxy_matched_dr_0p2
                .push(truth.truth_proxy_matched_dr_0p2);
            self.truth_proxy_matched_dr_0p3
                .push(truth.truth_proxy_matched_dr_0p3);
            self.truth_strategy_code.push(truth.truth_strategy.code());
            self.truth_photon_anchor_available
                .push(truth.truth_photon_anchor_available);
            self.gen_photon_from_higgs.push(truth.gen_photon_from_higgs);
            self.nearest_gen_pi_plus_available
                .push(truth.nearest_gen_pi_plus_available);
            self.nearest_gen_pi_minus_available
                .push(truth.nearest_gen_pi_minus_available);
            self.truth_matched.push(truth.truth_matched);
            self.truth_topology_code.push(truth.truth_topology.code());
            push_particle(
                &mut self.gen_h_pt,
                &mut self.gen_h_eta,
                &mut self.gen_h_phi,
                &mut self.gen_h_mass,
                truth.gen_h,
            );
            push_particle(
                &mut self.gen_rho_pt,
                &mut self.gen_rho_eta,
                &mut self.gen_rho_phi,
                &mut self.gen_rho_mass,
                truth.gen_rho,
            );
            push_particle(
                &mut self.gen_photon_pt,
                &mut self.gen_photon_eta,
                &mut self.gen_photon_phi,
                &mut self.gen_photon_mass,
                truth.gen_photon,
            );
            push_particle(
                &mut self.gen_pi_plus_pt,
                &mut self.gen_pi_plus_eta,
                &mut self.gen_pi_plus_phi,
                &mut self.gen_pi_plus_mass,
                truth.gen_pi_plus,
            );
            push_particle(
                &mut self.gen_pi_minus_pt,
                &mut self.gen_pi_minus_eta,
                &mut self.gen_pi_minus_phi,
                &mut self.gen_pi_minus_mass,
                truth.gen_pi_minus,
            );
            push_particle(
                &mut self.gen_rho_proxy_pt,
                &mut self.gen_rho_proxy_eta,
                &mut self.gen_rho_proxy_phi,
                &mut self.gen_rho_proxy_mass,
                truth.gen_rho_proxy,
            );
            push_particle(
                &mut self.gen_h_proxy_pt,
                &mut self.gen_h_proxy_eta,
                &mut self.gen_h_proxy_phi,
                &mut self.gen_h_proxy_mass,
                truth.gen_h_proxy,
            );
            self.delta_r_reco_photon_gen_photon
                .push(opt_f32(truth.delta_r_reco_photon_gen_photon));
            self.delta_r_reco_pi_plus_gen_pi_plus
                .push(opt_f32(truth.delta_r_reco_pi_plus_gen_pi_plus));
            self.delta_r_reco_pi_minus_gen_pi_minus
                .push(opt_f32(truth.delta_r_reco_pi_minus_gen_pi_minus));
            self.delta_r_reco_rho_gen_rho
                .push(opt_f32(truth.delta_r_reco_rho_gen_rho));
            self.delta_r_reco_rho_gen_rho_proxy
                .push(opt_f32(truth.delta_r_reco_rho_gen_rho_proxy));
            self.delta_r_reco_h_gen_h_proxy
                .push(opt_f32(truth.delta_r_reco_h_gen_h_proxy));
            self.reco_h_mass_minus_gen_h_mass
                .push(opt_f32(truth.reco_h_mass_minus_gen_h_mass));
            self.reco_rho_mass_minus_gen_rho_mass
                .push(opt_f32(truth.reco_rho_mass_minus_gen_rho_mass));
            self.reco_h_mass_minus_gen_h_proxy_mass
                .push(opt_f32(truth.reco_h_mass_minus_gen_h_proxy_mass));
            self.reco_rho_mass_minus_gen_rho_proxy_mass
                .push(opt_f32(truth.reco_rho_mass_minus_gen_rho_proxy_mass));
            self.reco_photon_pt_over_gen_photon_pt
                .push(opt_f32(truth.reco_photon_pt_over_gen_photon_pt));
            self.reco_rho_pt_over_gen_rho_pt
                .push(opt_f32(truth.reco_rho_pt_over_gen_rho_pt));
            self.reco_pi_plus_pt_over_gen_pi_plus_pt
                .push(opt_f32(truth.reco_pi_plus_pt_over_gen_pi_plus_pt));
            self.reco_pi_minus_pt_over_gen_pi_minus_pt
                .push(opt_f32(truth.reco_pi_minus_pt_over_gen_pi_minus_pt));
            self.reco_rho_pt_over_gen_rho_proxy_pt
                .push(opt_f32(truth.reco_rho_pt_over_gen_rho_proxy_pt));
            self.reco_h_pt_over_gen_h_proxy_pt
                .push(opt_f32(truth.reco_h_pt_over_gen_h_proxy_pt));
        }
    }

    fn save(&mut self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut branches = vec![
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
        if self.truth_enabled {
            branches.extend([
                OutputBranch::bool("truth_available", std::mem::take(&mut self.truth_available)),
                OutputBranch::bool(
                    "truth_proxy_matched",
                    std::mem::take(&mut self.truth_proxy_matched),
                ),
                OutputBranch::bool(
                    "truth_proxy_matched_dr_0p1",
                    std::mem::take(&mut self.truth_proxy_matched_dr_0p1),
                ),
                OutputBranch::bool(
                    "truth_proxy_matched_dr_0p2",
                    std::mem::take(&mut self.truth_proxy_matched_dr_0p2),
                ),
                OutputBranch::bool(
                    "truth_proxy_matched_dr_0p3",
                    std::mem::take(&mut self.truth_proxy_matched_dr_0p3),
                ),
                OutputBranch::i32(
                    "truth_strategy_code",
                    std::mem::take(&mut self.truth_strategy_code),
                ),
                OutputBranch::bool(
                    "truth_photon_anchor_available",
                    std::mem::take(&mut self.truth_photon_anchor_available),
                ),
                OutputBranch::bool(
                    "gen_photon_from_higgs",
                    std::mem::take(&mut self.gen_photon_from_higgs),
                ),
                OutputBranch::bool(
                    "nearest_gen_pi_plus_available",
                    std::mem::take(&mut self.nearest_gen_pi_plus_available),
                ),
                OutputBranch::bool(
                    "nearest_gen_pi_minus_available",
                    std::mem::take(&mut self.nearest_gen_pi_minus_available),
                ),
                OutputBranch::bool("truth_matched", std::mem::take(&mut self.truth_matched)),
                OutputBranch::i32(
                    "truth_topology_code",
                    std::mem::take(&mut self.truth_topology_code),
                ),
                OutputBranch::f32("gen_h_pt", std::mem::take(&mut self.gen_h_pt)),
                OutputBranch::f32("gen_h_eta", std::mem::take(&mut self.gen_h_eta)),
                OutputBranch::f32("gen_h_phi", std::mem::take(&mut self.gen_h_phi)),
                OutputBranch::f32("gen_h_mass", std::mem::take(&mut self.gen_h_mass)),
                OutputBranch::f32("gen_rho_pt", std::mem::take(&mut self.gen_rho_pt)),
                OutputBranch::f32("gen_rho_eta", std::mem::take(&mut self.gen_rho_eta)),
                OutputBranch::f32("gen_rho_phi", std::mem::take(&mut self.gen_rho_phi)),
                OutputBranch::f32("gen_rho_mass", std::mem::take(&mut self.gen_rho_mass)),
                OutputBranch::f32("gen_photon_pt", std::mem::take(&mut self.gen_photon_pt)),
                OutputBranch::f32("gen_photon_eta", std::mem::take(&mut self.gen_photon_eta)),
                OutputBranch::f32("gen_photon_phi", std::mem::take(&mut self.gen_photon_phi)),
                OutputBranch::f32("gen_photon_mass", std::mem::take(&mut self.gen_photon_mass)),
                OutputBranch::f32("gen_pi_plus_pt", std::mem::take(&mut self.gen_pi_plus_pt)),
                OutputBranch::f32("gen_pi_plus_eta", std::mem::take(&mut self.gen_pi_plus_eta)),
                OutputBranch::f32("gen_pi_plus_phi", std::mem::take(&mut self.gen_pi_plus_phi)),
                OutputBranch::f32(
                    "gen_pi_plus_mass",
                    std::mem::take(&mut self.gen_pi_plus_mass),
                ),
                OutputBranch::f32("gen_pi_minus_pt", std::mem::take(&mut self.gen_pi_minus_pt)),
                OutputBranch::f32(
                    "gen_pi_minus_eta",
                    std::mem::take(&mut self.gen_pi_minus_eta),
                ),
                OutputBranch::f32(
                    "gen_pi_minus_phi",
                    std::mem::take(&mut self.gen_pi_minus_phi),
                ),
                OutputBranch::f32(
                    "gen_pi_minus_mass",
                    std::mem::take(&mut self.gen_pi_minus_mass),
                ),
                OutputBranch::f32(
                    "gen_rho_proxy_pt",
                    std::mem::take(&mut self.gen_rho_proxy_pt),
                ),
                OutputBranch::f32(
                    "gen_rho_proxy_eta",
                    std::mem::take(&mut self.gen_rho_proxy_eta),
                ),
                OutputBranch::f32(
                    "gen_rho_proxy_phi",
                    std::mem::take(&mut self.gen_rho_proxy_phi),
                ),
                OutputBranch::f32(
                    "gen_rho_proxy_mass",
                    std::mem::take(&mut self.gen_rho_proxy_mass),
                ),
                OutputBranch::f32("gen_h_proxy_pt", std::mem::take(&mut self.gen_h_proxy_pt)),
                OutputBranch::f32("gen_h_proxy_eta", std::mem::take(&mut self.gen_h_proxy_eta)),
                OutputBranch::f32("gen_h_proxy_phi", std::mem::take(&mut self.gen_h_proxy_phi)),
                OutputBranch::f32(
                    "gen_h_proxy_mass",
                    std::mem::take(&mut self.gen_h_proxy_mass),
                ),
                OutputBranch::f32(
                    "delta_r_reco_photon_gen_photon",
                    std::mem::take(&mut self.delta_r_reco_photon_gen_photon),
                ),
                OutputBranch::f32(
                    "delta_r_reco_pi_plus_gen_pi_plus",
                    std::mem::take(&mut self.delta_r_reco_pi_plus_gen_pi_plus),
                ),
                OutputBranch::f32(
                    "delta_r_reco_pi_minus_gen_pi_minus",
                    std::mem::take(&mut self.delta_r_reco_pi_minus_gen_pi_minus),
                ),
                OutputBranch::f32(
                    "delta_r_reco_rho_gen_rho",
                    std::mem::take(&mut self.delta_r_reco_rho_gen_rho),
                ),
                OutputBranch::f32(
                    "delta_r_reco_rho_gen_rho_proxy",
                    std::mem::take(&mut self.delta_r_reco_rho_gen_rho_proxy),
                ),
                OutputBranch::f32(
                    "delta_r_reco_h_gen_h_proxy",
                    std::mem::take(&mut self.delta_r_reco_h_gen_h_proxy),
                ),
                OutputBranch::f32(
                    "reco_h_mass_minus_gen_h_mass",
                    std::mem::take(&mut self.reco_h_mass_minus_gen_h_mass),
                ),
                OutputBranch::f32(
                    "reco_rho_mass_minus_gen_rho_mass",
                    std::mem::take(&mut self.reco_rho_mass_minus_gen_rho_mass),
                ),
                OutputBranch::f32(
                    "reco_h_mass_minus_gen_h_proxy_mass",
                    std::mem::take(&mut self.reco_h_mass_minus_gen_h_proxy_mass),
                ),
                OutputBranch::f32(
                    "reco_rho_mass_minus_gen_rho_proxy_mass",
                    std::mem::take(&mut self.reco_rho_mass_minus_gen_rho_proxy_mass),
                ),
                OutputBranch::f32(
                    "reco_photon_pt_over_gen_photon_pt",
                    std::mem::take(&mut self.reco_photon_pt_over_gen_photon_pt),
                ),
                OutputBranch::f32(
                    "reco_rho_pt_over_gen_rho_pt",
                    std::mem::take(&mut self.reco_rho_pt_over_gen_rho_pt),
                ),
                OutputBranch::f32(
                    "reco_pi_plus_pt_over_gen_pi_plus_pt",
                    std::mem::take(&mut self.reco_pi_plus_pt_over_gen_pi_plus_pt),
                ),
                OutputBranch::f32(
                    "reco_pi_minus_pt_over_gen_pi_minus_pt",
                    std::mem::take(&mut self.reco_pi_minus_pt_over_gen_pi_minus_pt),
                ),
                OutputBranch::f32(
                    "reco_rho_pt_over_gen_rho_proxy_pt",
                    std::mem::take(&mut self.reco_rho_pt_over_gen_rho_proxy_pt),
                ),
                OutputBranch::f32(
                    "reco_h_pt_over_gen_h_proxy_pt",
                    std::mem::take(&mut self.reco_h_pt_over_gen_h_proxy_pt),
                ),
            ]);
        }
        write_events(path, &branches)?;
        Ok(())
    }
}

fn opt_f32(value: Option<f64>) -> f32 {
    value.map(|value| value as f32).unwrap_or(f32::NAN)
}

fn push_particle(
    pt: &mut Vec<f32>,
    eta: &mut Vec<f32>,
    phi: &mut Vec<f32>,
    mass: &mut Vec<f32>,
    particle: Option<GenParticle>,
) {
    pt.push(opt_f32(particle.map(|particle| particle.pt)));
    eta.push(opt_f32(particle.map(|particle| particle.eta)));
    phi.push(opt_f32(particle.map(|particle| particle.phi)));
    mass.push(opt_f32(particle.map(|particle| particle.mass)));
}
