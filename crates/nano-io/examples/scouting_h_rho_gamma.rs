use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

use nano_core::{BranchSchema, BranchSpec, BranchType};
use nano_io::events_chunked;
use nano_io::scouting_hrhogamma::{reconstruct_event, EventInputs, HCand, HToRhoGammaCuts};

const ENV_INPUT: &str = "NANO_SCOUTING_HRHOGAMMA_FILE";
const CHUNK_SIZE: usize = 1024;
const MAX_PRINTED_CANDIDATES: usize = 10;

fn main() -> Result<(), Box<dyn Error>> {
    let Some(options) = Options::parse()? else {
        print_usage();
        return Ok(());
    };

    let schema = scouting_schema()?;
    let cuts = HToRhoGammaCuts::zcountinghlt_naive();
    println!("input: {}", options.input.display());
    println!("max_events: {}", display_limit(options.max_events));
    println!("branch_schema: ok");
    println!("branch_mapping: ScoutingPhoton=Photon_*, ScoutingChargedCandidate=PFCand_*");
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

    let report = analyze(&options.input, &schema, &cuts, options.max_events)?;
    print_report(&report);
    Ok(())
}

#[derive(Debug)]
struct Options {
    input: PathBuf,
    max_events: Option<usize>,
}

impl Options {
    fn parse() -> Result<Option<Self>, Box<dyn Error>> {
        let mut positional = env::args().skip(1).collect::<Vec<_>>();
        if positional.iter().any(|arg| arg == "-h" || arg == "--help") {
            return Ok(None);
        }
        if positional.len() > 2 {
            return Err("usage: scouting_h_rho_gamma <input.root> [max-events]".into());
        }

        let input = if positional.is_empty() {
            match env::var(ENV_INPUT) {
                Ok(value) => PathBuf::from(value),
                Err(_) => return Ok(None),
            }
        } else {
            PathBuf::from(positional.remove(0))
        };
        let max_events = positional
            .first()
            .map(|value| value.parse::<usize>())
            .transpose()
            .map_err(|err| format!("invalid max event count: {err}"))?;

        Ok(Some(Self { input, max_events }))
    }
}

fn print_usage() {
    println!("usage: scouting_h_rho_gamma <input.root> [max-events]");
    println!("or set {ENV_INPUT}=<input.root>");
}

fn display_limit(limit: Option<usize>) -> String {
    limit
        .map(|value| value.to_string())
        .unwrap_or_else(|| "all".to_string())
}

fn scouting_schema() -> Result<BranchSchema, Box<dyn Error>> {
    Ok(BranchSchema::new([
        BranchSpec::new("run", BranchType::U32),
        BranchSpec::new("luminosityBlock", BranchType::U32),
        BranchSpec::new("event", BranchType::U64),
        BranchSpec::new("nPhoton", BranchType::I32),
        BranchSpec::new("Photon_pt", BranchType::VecF32),
        BranchSpec::new("Photon_eta", BranchType::VecF32),
        BranchSpec::new("Photon_phi", BranchType::VecF32),
        BranchSpec::new("nPFCand", BranchType::I32),
        BranchSpec::new("PFCand_pt", BranchType::VecF32),
        BranchSpec::new("PFCand_eta", BranchType::VecF32),
        BranchSpec::new("PFCand_phi", BranchType::VecF32),
        BranchSpec::new("PFCand_pdgId", BranchType::VecI32),
        BranchSpec::new("PFCand_mass", BranchType::VecF32).optional(),
    ])?)
}

fn analyze(
    input: &Path,
    schema: &BranchSchema,
    cuts: &HToRhoGammaCuts,
    max_events: Option<usize>,
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

        let n_photon = nonnegative_count(event.scalar::<i32>("nPhoton")?, "nPhoton")?;
        let photon_pt = event.vector_ref::<f32>("Photon_pt")?;
        let photon_eta = event.vector_ref::<f32>("Photon_eta")?;
        let photon_phi = event.vector_ref::<f32>("Photon_phi")?;
        validate_len("Photon_pt", photon_pt.len(), n_photon)?;
        validate_len("Photon_eta", photon_eta.len(), n_photon)?;
        validate_len("Photon_phi", photon_phi.len(), n_photon)?;

        let n_pfcand = nonnegative_count(event.scalar::<i32>("nPFCand")?, "nPFCand")?;
        let pfcand_pt = event.vector_ref::<f32>("PFCand_pt")?;
        let pfcand_eta = event.vector_ref::<f32>("PFCand_eta")?;
        let pfcand_phi = event.vector_ref::<f32>("PFCand_phi")?;
        let pfcand_pdg_id = event.vector_ref::<i32>("PFCand_pdgId")?;
        validate_len("PFCand_pt", pfcand_pt.len(), n_pfcand)?;
        validate_len("PFCand_eta", pfcand_eta.len(), n_pfcand)?;
        validate_len("PFCand_phi", pfcand_phi.len(), n_pfcand)?;
        validate_len("PFCand_pdgId", pfcand_pdg_id.len(), n_pfcand)?;

        let pfcand_mass = if event.has_physical_branch("PFCand_mass") {
            let mass = event.vector_ref::<f32>("PFCand_mass")?;
            validate_len("PFCand_mass", mass.len(), n_pfcand)?;
            Some(mass)
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

        if report.candidates.len() < MAX_PRINTED_CANDIDATES {
            let gamma_rho_delta_r = reco
                .gamma_rho_delta_r
                .expect("H candidate requires gamma-rho deltaR");
            report.candidates.push(CandidateSummary {
                run,
                luminosity_block,
                event: event_number,
                gamma_rho_delta_r,
                rho_over_gamma_pt: h.rho.pt / h.gamma.pt,
                h,
            });
        }
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
