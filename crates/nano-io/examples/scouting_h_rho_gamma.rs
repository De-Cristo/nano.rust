use std::cmp::Ordering;
use std::env;
use std::error::Error;
use std::path::{Path, PathBuf};

use nano_core::{BranchSchema, BranchSpec, BranchType};
use nano_io::events_chunked;

const ENV_INPUT: &str = "NANO_SCOUTING_HRHOGAMMA_FILE";
const CHUNK_SIZE: usize = 1024;
const MAX_PRINTED_CANDIDATES: usize = 10;

const M_PI_CHARGED: f64 = 0.139_570_39;
const M_RHO_TARGET: f64 = 0.775_26;
const M_HIGGS_REFERENCE: f64 = 125.0;

const NAIVE_PHO_MIN_PT: f64 = 15.0;
const NAIVE_PI1_MIN_PT: f64 = 5.0;
const NAIVE_PI2_MIN_PT: f64 = 2.0;
const NAIVE_MAX_DR_PIPI: f64 = 0.1;
const NAIVE_RHO_MASS_MIN: f64 = 0.3;
const NAIVE_RHO_MASS_MAX: f64 = 1.2;
const NAIVE_MIN_DR_G_RHO: f64 = 1.0;
const NAIVE_MAX_DR_G_RHO: f64 = 5.0;

fn main() -> Result<(), Box<dyn Error>> {
    let Some(options) = Options::parse()? else {
        print_usage();
        return Ok(());
    };

    let schema = scouting_schema()?;
    println!("input: {}", options.input.display());
    println!("max_events: {}", display_limit(options.max_events));
    println!("branch_schema: ok");
    println!("branch_mapping: ScoutingPhoton=Photon_*, ScoutingChargedCandidate=PFCand_*");
    println!(
        "constants: m_pi_charged={M_PI_CHARGED:.8} m_rho_target={M_RHO_TARGET:.5} m_higgs_reference={M_HIGGS_REFERENCE:.1}"
    );
    println!(
        "cuts: photon_pt>={NAIVE_PHO_MIN_PT:.1} pi1_pt>={NAIVE_PI1_MIN_PT:.1} pi2_pt>={NAIVE_PI2_MIN_PT:.1} dr_pipi<{NAIVE_MAX_DR_PIPI:.2} rho_mass=[{NAIVE_RHO_MASS_MIN:.1},{NAIVE_RHO_MASS_MAX:.1}] dr_gamma_rho=[{NAIVE_MIN_DR_G_RHO:.1},{NAIVE_MAX_DR_G_RHO:.1}]"
    );

    let report = analyze(&options.input, &schema, options.max_events)?;
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

        let photon = select_leading_photon(photon_pt, photon_eta, photon_phi);
        let mut pions = collect_pions(
            pfcand_pt,
            pfcand_eta,
            pfcand_phi,
            pfcand_pdg_id,
            pfcand_mass,
            &mut report,
        );

        let Some(photon) = photon else {
            continue;
        };
        report.cutflow.leading_photon += 1;

        if pions.len() < 2 {
            continue;
        }
        report.cutflow.two_pions += 1;

        pions.sort_by(|a, b| b.pt.partial_cmp(&a.pt).unwrap_or(Ordering::Equal));
        let PairSearch {
            has_os_pt_pair,
            has_dr_pair,
            best_rho,
        } = find_best_rho(&pions);

        if !has_os_pt_pair {
            continue;
        }
        report.cutflow.os_pt_pair += 1;

        if !has_dr_pair {
            continue;
        }
        report.cutflow.pipi_delta_r += 1;

        let Some(rho) = best_rho else {
            continue;
        };
        report.cutflow.rho_mass_window += 1;

        let gamma_rho_delta_r = delta_r(photon.eta, photon.phi, rho.eta, rho.phi);
        if !(NAIVE_MIN_DR_G_RHO..=NAIVE_MAX_DR_G_RHO).contains(&gamma_rho_delta_r) {
            continue;
        }
        report.cutflow.gamma_rho_delta_r += 1;

        let h_p4 = photon.p4.add(&rho.p4);
        let h = HCand {
            gamma: photon,
            rho,
            p4: h_p4,
            mass: h_p4.mass(),
            pt: h_p4.pt(),
            eta: h_p4.eta(),
            phi: h_p4.phi(),
        };
        report.cutflow.h_candidate += 1;

        if report.candidates.len() < MAX_PRINTED_CANDIDATES {
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

fn select_leading_photon(pt: &[f32], eta: &[f32], phi: &[f32]) -> Option<SimpleCand> {
    pt.iter()
        .zip(eta)
        .zip(phi)
        .filter_map(|((&pt, &eta), &phi)| {
            let pt = f64::from(pt);
            (pt >= NAIVE_PHO_MIN_PT)
                .then(|| SimpleCand::new(pt, f64::from(eta), f64::from(phi), 0.0, 0))
        })
        .max_by(|a, b| a.pt.partial_cmp(&b.pt).unwrap_or(Ordering::Equal))
}

fn collect_pions(
    pt: &[f32],
    eta: &[f32],
    phi: &[f32],
    pdg_id: &[i32],
    mass: Option<&[f32]>,
    report: &mut Report,
) -> Vec<SimpleCand> {
    let mut pions = Vec::new();
    for index in 0..pdg_id.len() {
        match pdg_id[index] {
            211 => report.plus_pions += 1,
            -211 => report.minus_pions += 1,
            _ => {}
        }

        if pdg_id[index].abs() != 211 {
            continue;
        }
        let charge = pdg_id[index].signum();
        if charge == 0 {
            continue;
        }
        let pt = f64::from(pt[index]);
        if pt < NAIVE_PI2_MIN_PT {
            continue;
        }
        if let Some(mass) = mass {
            report.pion_mass_sum += f64::from(mass[index]);
            report.pion_mass_count += 1;
        }
        pions.push(SimpleCand::new(
            pt,
            f64::from(eta[index]),
            f64::from(phi[index]),
            M_PI_CHARGED,
            charge,
        ));
    }
    pions
}

#[derive(Debug)]
struct PairSearch {
    has_os_pt_pair: bool,
    has_dr_pair: bool,
    best_rho: Option<RhoCand>,
}

fn find_best_rho(pions: &[SimpleCand]) -> PairSearch {
    let mut has_os_pt_pair = false;
    let mut has_dr_pair = false;
    let mut best: Option<(f64, RhoCand)> = None;

    for i in 0..pions.len() {
        for j in (i + 1)..pions.len() {
            let leading = &pions[i];
            let subleading = &pions[j];
            if leading.pt < NAIVE_PI1_MIN_PT || subleading.pt < NAIVE_PI2_MIN_PT {
                continue;
            }
            if leading.charge * subleading.charge >= 0 {
                continue;
            }
            has_os_pt_pair = true;

            let pipi_delta_r = delta_r(leading.eta, leading.phi, subleading.eta, subleading.phi);
            if pipi_delta_r >= NAIVE_MAX_DR_PIPI {
                continue;
            }
            has_dr_pair = true;

            let rho = RhoCand::from_pair(leading.clone(), subleading.clone(), pipi_delta_r);
            if !(NAIVE_RHO_MASS_MIN..=NAIVE_RHO_MASS_MAX).contains(&rho.mass) {
                continue;
            }

            let distance = (rho.mass - M_RHO_TARGET).abs();
            if best
                .as_ref()
                .map(|(best_distance, _)| distance < *best_distance)
                .unwrap_or(true)
            {
                best = Some((distance, rho));
            }
        }
    }

    PairSearch {
        has_os_pt_pair,
        has_dr_pair,
        best_rho: best.map(|(_, rho)| rho),
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

#[derive(Debug, Clone)]
struct SimpleCand {
    pt: f64,
    eta: f64,
    phi: f64,
    mass: f64,
    charge: i32,
    p4: FourVec,
}

impl SimpleCand {
    fn new(pt: f64, eta: f64, phi: f64, mass: f64, charge: i32) -> Self {
        Self {
            pt,
            eta,
            phi,
            mass,
            charge,
            p4: FourVec::from_pt_eta_phi_mass(pt, eta, phi, mass),
        }
    }
}

#[derive(Debug, Clone)]
struct RhoCand {
    pi_plus: SimpleCand,
    pi_minus: SimpleCand,
    p4: FourVec,
    mass: f64,
    pt: f64,
    eta: f64,
    phi: f64,
    pipi_delta_r: f64,
}

impl RhoCand {
    fn from_pair(first: SimpleCand, second: SimpleCand, pipi_delta_r: f64) -> Self {
        let (pi_plus, pi_minus) = if first.charge > 0 {
            (first, second)
        } else {
            (second, first)
        };
        let p4 = pi_plus.p4.add(&pi_minus.p4);
        Self {
            pi_plus,
            pi_minus,
            p4,
            mass: p4.mass(),
            pt: p4.pt(),
            eta: p4.eta(),
            phi: p4.phi(),
            pipi_delta_r,
        }
    }
}

#[derive(Debug, Clone)]
struct HCand {
    gamma: SimpleCand,
    rho: RhoCand,
    p4: FourVec,
    mass: f64,
    pt: f64,
    eta: f64,
    phi: f64,
}

#[derive(Debug, Clone, Copy)]
struct FourVec {
    px: f64,
    py: f64,
    pz: f64,
    e: f64,
}

impl FourVec {
    fn from_pt_eta_phi_mass(pt: f64, eta: f64, phi: f64, mass: f64) -> Self {
        let px = pt * phi.cos();
        let py = pt * phi.sin();
        let pz = pt * eta.sinh();
        let p2 = px * px + py * py + pz * pz;
        let e = (p2 + mass * mass).sqrt();
        Self { px, py, pz, e }
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            px: self.px + other.px,
            py: self.py + other.py,
            pz: self.pz + other.pz,
            e: self.e + other.e,
        }
    }

    fn pt(&self) -> f64 {
        self.px.hypot(self.py)
    }

    fn eta(&self) -> f64 {
        let pt = self.pt();
        if pt > 0.0 {
            (self.pz / pt).asinh()
        } else {
            0.0
        }
    }

    fn phi(&self) -> f64 {
        self.py.atan2(self.px)
    }

    fn mass(&self) -> f64 {
        let p2 = self.px * self.px + self.py * self.py + self.pz * self.pz;
        (self.e * self.e - p2).max(0.0).sqrt()
    }
}

fn delta_r(eta_a: f64, phi_a: f64, eta_b: f64, phi_b: f64) -> f64 {
    let deta = eta_a - eta_b;
    let dphi = delta_phi(phi_a, phi_b);
    deta.hypot(dphi)
}

fn delta_phi(phi_a: f64, phi_b: f64) -> f64 {
    let mut dphi = phi_a - phi_b;
    while dphi > std::f64::consts::PI {
        dphi -= 2.0 * std::f64::consts::PI;
    }
    while dphi <= -std::f64::consts::PI {
        dphi += 2.0 * std::f64::consts::PI;
    }
    dphi
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
