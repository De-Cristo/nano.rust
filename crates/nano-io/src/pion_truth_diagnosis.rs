use std::collections::BTreeMap;

use serde::Serialize;

use crate::genpart_survey::GenPartRecord;
use crate::h_rho_gamma::delta_r;

pub const THRESHOLDS: [f64; 5] = [0.05, 0.10, 0.20, 0.30, 0.50];
pub const POOLS: [&str; 5] = [
    "pi_only_same_charge",
    "pi_only_opposite_charge",
    "any_charged_hadron_same_charge",
    "any_charged_stable_like_same_charge",
    "any_charged_particle_same_charge",
];

#[derive(Debug, Clone, PartialEq)]
pub struct RecoCandidate {
    pub run: u32,
    pub luminosity_block: u32,
    pub event: u64,
    pub pi_plus_pt: f64,
    pub pi_plus_eta: f64,
    pub pi_plus_phi: f64,
    pub pi_minus_pt: f64,
    pub pi_minus_eta: f64,
    pub pi_minus_phi: f64,
    pub h_mass: f64,
    pub rho_mass: f64,
    pub rho_pt_over_photon_pt: f64,
    pub photon_matched: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CandidateDiagnostic {
    pub run: u32,
    #[serde(rename = "luminosityBlock")]
    pub luminosity_block: u32,
    pub event: u64,
    pub h_mass: f64,
    pub rho_mass: f64,
    pub rho_pt_over_photon_pt: f64,
    pub pi_plus_pt: f64,
    pub pi_minus_pt: f64,
    pub pi_plus_eta: f64,
    pub pi_minus_eta: f64,
    pub nearest_pi_plus_dr: Option<f64>,
    pub nearest_pi_minus_dr: Option<f64>,
    pub nearest_any_charged_hadron_plus_dr: Option<f64>,
    pub nearest_any_charged_hadron_minus_dr: Option<f64>,
    pub nearest_any_charged_stable_like_plus_dr: Option<f64>,
    pub nearest_any_charged_stable_like_minus_dr: Option<f64>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PionTruthDiagnosisSummary {
    pub candidate_rows: u64,
    pub diagnosed_candidates: u64,
    pub pool_event_counts: BTreeMap<String, u64>,
    pub pool_particle_counts: BTreeMap<String, u64>,
    pub leg_threshold_counts: BTreeMap<String, BTreeMap<String, BTreeMap<String, u64>>>,
    pub both_threshold_counts: BTreeMap<String, BTreeMap<String, u64>>,
    pub category_counts: BTreeMap<String, u64>,
    pub nearest_dr: BTreeMap<String, Vec<f64>>,
    pub candidate_diagnostics: Vec<CandidateDiagnostic>,
    pub packed_genpart_branches_present: bool,
}

impl PionTruthDiagnosisSummary {
    pub fn new(candidate_rows: u64) -> Self {
        let mut summary = Self {
            candidate_rows,
            diagnosed_candidates: 0,
            pool_event_counts: BTreeMap::new(),
            pool_particle_counts: BTreeMap::new(),
            leg_threshold_counts: BTreeMap::new(),
            both_threshold_counts: BTreeMap::new(),
            category_counts: BTreeMap::new(),
            nearest_dr: BTreeMap::new(),
            candidate_diagnostics: Vec::new(),
            packed_genpart_branches_present: false,
        };
        for leg in ["pi_plus_leg", "pi_minus_leg"] {
            let mut pool_map = BTreeMap::new();
            for pool in POOLS {
                pool_map.insert(pool.to_string(), threshold_map());
            }
            summary
                .leg_threshold_counts
                .insert(leg.to_string(), pool_map);
        }
        for pool in POOLS {
            summary
                .both_threshold_counts
                .insert(pool.to_string(), threshold_map());
        }
        for name in POOL_BREAKDOWN_NAMES {
            summary.pool_event_counts.insert(name.to_string(), 0);
            summary.pool_particle_counts.insert(name.to_string(), 0);
        }
        for name in NEAREST_DR_NAMES {
            summary.nearest_dr.insert(name.to_string(), Vec::new());
        }
        for name in CATEGORY_NAMES {
            summary.category_counts.insert(name.to_string(), 0);
        }
        summary
    }

    pub fn add_event_candidates(
        &mut self,
        candidates: &[RecoCandidate],
        particles: &[GenPartRecord],
    ) {
        add_pool_breakdown(self, particles);
        for candidate in candidates {
            self.add_candidate(candidate, particles);
        }
    }

    pub fn add_candidate(&mut self, candidate: &RecoCandidate, particles: &[GenPartRecord]) {
        self.diagnosed_candidates += 1;

        let plus_matches = leg_matches(candidate.pi_plus_eta, candidate.pi_plus_phi, 1, particles);
        let minus_matches = leg_matches(
            candidate.pi_minus_eta,
            candidate.pi_minus_phi,
            -1,
            particles,
        );

        add_leg_threshold_counts(self, "pi_plus_leg", &plus_matches);
        add_leg_threshold_counts(self, "pi_minus_leg", &minus_matches);
        add_both_threshold_counts(self, &plus_matches, &minus_matches);
        add_nearest_dr(self, &plus_matches, &minus_matches);

        let categories = classify_candidate(candidate, &plus_matches, &minus_matches);
        for category in &categories {
            *self.category_counts.entry(category.clone()).or_default() += 1;
        }

        self.candidate_diagnostics.push(CandidateDiagnostic {
            run: candidate.run,
            luminosity_block: candidate.luminosity_block,
            event: candidate.event,
            h_mass: candidate.h_mass,
            rho_mass: candidate.rho_mass,
            rho_pt_over_photon_pt: candidate.rho_pt_over_photon_pt,
            pi_plus_pt: candidate.pi_plus_pt,
            pi_minus_pt: candidate.pi_minus_pt,
            pi_plus_eta: candidate.pi_plus_eta,
            pi_minus_eta: candidate.pi_minus_eta,
            nearest_pi_plus_dr: plus_matches.get("pi_only_same_charge").copied().flatten(),
            nearest_pi_minus_dr: minus_matches.get("pi_only_same_charge").copied().flatten(),
            nearest_any_charged_hadron_plus_dr: plus_matches
                .get("any_charged_hadron_same_charge")
                .copied()
                .flatten(),
            nearest_any_charged_hadron_minus_dr: minus_matches
                .get("any_charged_hadron_same_charge")
                .copied()
                .flatten(),
            nearest_any_charged_stable_like_plus_dr: plus_matches
                .get("any_charged_stable_like_same_charge")
                .copied()
                .flatten(),
            nearest_any_charged_stable_like_minus_dr: minus_matches
                .get("any_charged_stable_like_same_charge")
                .copied()
                .flatten(),
            categories,
        });
    }
}

const POOL_BREAKDOWN_NAMES: [&str; 20] = [
    "all_pi_plus",
    "all_pi_minus",
    "finite_pi_plus",
    "finite_pi_minus",
    "status1_pi_plus",
    "status1_pi_minus",
    "status1_finite_pi_plus",
    "status1_finite_pi_minus",
    "pt_gt_0p1_pi_plus",
    "pt_gt_0p1_pi_minus",
    "pt_gt_0p5_pi_plus",
    "pt_gt_0p5_pi_minus",
    "pt_gt_1p0_pi_plus",
    "pt_gt_1p0_pi_minus",
    "eta_lt_2p5_pi_plus",
    "eta_lt_2p5_pi_minus",
    "charged_pions",
    "charged_kaons",
    "protons",
    "charged_leptons",
];

const NEAREST_DR_NAMES: [&str; 6] = [
    "nearest_pi_plus_dr",
    "nearest_pi_minus_dr",
    "nearest_any_charged_hadron_plus_dr",
    "nearest_any_charged_hadron_minus_dr",
    "nearest_any_charged_stable_like_plus_dr",
    "nearest_any_charged_stable_like_minus_dr",
];

const CATEGORY_NAMES: [&str; 7] = [
    "both_reco_pions_match_gen_pions_dr0p1",
    "only_plus_matches_gen_pion_dr0p1",
    "only_minus_matches_gen_pion_dr0p1",
    "no_gen_pion_match_dr0p1",
    "both_match_any_charged_hadron_dr0p1",
    "both_match_any_charged_stable_like_dr0p1",
    "photon_matched_but_no_pion_match",
];

fn threshold_map() -> BTreeMap<String, u64> {
    THRESHOLDS
        .iter()
        .map(|threshold| (format_threshold(*threshold), 0))
        .collect()
}

fn format_threshold(threshold: f64) -> String {
    format!("{threshold:.2}")
}

fn add_pool_breakdown(summary: &mut PionTruthDiagnosisSummary, particles: &[GenPartRecord]) {
    let mut event_counts: BTreeMap<&str, u64> = BTreeMap::new();
    for particle in particles {
        for pool in breakdown_pools(particle) {
            *summary
                .pool_particle_counts
                .entry(pool.to_string())
                .or_default() += 1;
            event_counts.insert(pool, 1);
        }
    }
    for pool in event_counts.keys() {
        *summary
            .pool_event_counts
            .entry((*pool).to_string())
            .or_default() += 1;
    }
}

fn breakdown_pools(particle: &GenPartRecord) -> Vec<&'static str> {
    let mut pools = Vec::new();
    let finite = finite_eta_phi(particle);
    match particle.pdg_id {
        211 => {
            pools.push("all_pi_plus");
            if finite {
                pools.push("finite_pi_plus");
            }
            if particle.status == 1 {
                pools.push("status1_pi_plus");
                if finite {
                    pools.push("status1_finite_pi_plus");
                }
            }
            if particle.pt > 0.1 {
                pools.push("pt_gt_0p1_pi_plus");
            }
            if particle.pt > 0.5 {
                pools.push("pt_gt_0p5_pi_plus");
            }
            if particle.pt > 1.0 {
                pools.push("pt_gt_1p0_pi_plus");
            }
            if particle.eta.abs() < 2.5 {
                pools.push("eta_lt_2p5_pi_plus");
            }
            pools.push("charged_pions");
        }
        -211 => {
            pools.push("all_pi_minus");
            if finite {
                pools.push("finite_pi_minus");
            }
            if particle.status == 1 {
                pools.push("status1_pi_minus");
                if finite {
                    pools.push("status1_finite_pi_minus");
                }
            }
            if particle.pt > 0.1 {
                pools.push("pt_gt_0p1_pi_minus");
            }
            if particle.pt > 0.5 {
                pools.push("pt_gt_0p5_pi_minus");
            }
            if particle.pt > 1.0 {
                pools.push("pt_gt_1p0_pi_minus");
            }
            if particle.eta.abs() < 2.5 {
                pools.push("eta_lt_2p5_pi_minus");
            }
            pools.push("charged_pions");
        }
        321 | -321 => pools.push("charged_kaons"),
        2212 | -2212 => pools.push("protons"),
        11 | -11 | 13 | -13 => pools.push("charged_leptons"),
        _ => {}
    }
    if is_charged_hadron(particle.pdg_id) {
        pools.push("any_charged_hadron");
    }
    if is_charged_stable_like(particle) {
        pools.push("any_charged_stable_like");
    }
    pools
}

fn leg_matches(
    reco_eta: f64,
    reco_phi: f64,
    charge: i32,
    particles: &[GenPartRecord],
) -> BTreeMap<String, Option<f64>> {
    let mut matches = BTreeMap::new();
    for pool in POOLS {
        let nearest = particles
            .iter()
            .filter(|particle| finite_eta_phi(particle))
            .filter(|particle| match_pool(pool, charge, particle))
            .map(|particle| delta_r(reco_eta, reco_phi, particle.eta, particle.phi))
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        matches.insert(pool.to_string(), nearest);
    }
    matches
}

fn match_pool(pool: &str, charge: i32, particle: &GenPartRecord) -> bool {
    match pool {
        "pi_only_same_charge" => particle.pdg_id == 211 * charge,
        "pi_only_opposite_charge" => particle.pdg_id == -211 * charge,
        "any_charged_hadron_same_charge" => {
            same_charge(charge, particle.pdg_id) && is_charged_hadron(particle.pdg_id)
        }
        "any_charged_stable_like_same_charge" => {
            same_charge(charge, particle.pdg_id) && is_charged_stable_like(particle)
        }
        "any_charged_particle_same_charge" => {
            same_charge(charge, particle.pdg_id) && is_known_charged(particle.pdg_id)
        }
        _ => false,
    }
}

fn same_charge(charge: i32, pdg_id: i32) -> bool {
    pdg_charge_sign(pdg_id).is_some_and(|sign| sign == charge.signum())
}

fn pdg_charge_sign(pdg_id: i32) -> Option<i32> {
    match pdg_id.abs() {
        11 | 13 => Some(-pdg_id.signum()),
        211 | 321 | 2212 => Some(pdg_id.signum()),
        _ => None,
    }
}

fn finite_eta_phi(particle: &GenPartRecord) -> bool {
    particle.pt > 0.0 && particle.eta.is_finite() && particle.phi.is_finite()
}

fn is_known_charged(pdg_id: i32) -> bool {
    matches!(pdg_id.abs(), 11 | 13 | 211 | 321 | 2212)
}

fn is_charged_hadron(pdg_id: i32) -> bool {
    matches!(pdg_id.abs(), 211 | 321 | 2212)
}

fn is_charged_stable_like(particle: &GenPartRecord) -> bool {
    particle.status == 1 && is_known_charged(particle.pdg_id) && finite_eta_phi(particle)
}

fn add_leg_threshold_counts(
    summary: &mut PionTruthDiagnosisSummary,
    leg: &str,
    matches: &BTreeMap<String, Option<f64>>,
) {
    for (pool, nearest) in matches {
        let Some(nearest) = nearest else {
            continue;
        };
        for threshold in THRESHOLDS {
            if *nearest < threshold {
                *summary
                    .leg_threshold_counts
                    .entry(leg.to_string())
                    .or_default()
                    .entry(pool.clone())
                    .or_default()
                    .entry(format_threshold(threshold))
                    .or_default() += 1;
            }
        }
    }
}

fn add_both_threshold_counts(
    summary: &mut PionTruthDiagnosisSummary,
    plus: &BTreeMap<String, Option<f64>>,
    minus: &BTreeMap<String, Option<f64>>,
) {
    for pool in POOLS {
        let Some(Some(plus_dr)) = plus.get(pool) else {
            continue;
        };
        let Some(Some(minus_dr)) = minus.get(pool) else {
            continue;
        };
        for threshold in THRESHOLDS {
            if *plus_dr < threshold && *minus_dr < threshold {
                *summary
                    .both_threshold_counts
                    .entry(pool.to_string())
                    .or_default()
                    .entry(format_threshold(threshold))
                    .or_default() += 1;
            }
        }
    }
}

fn add_nearest_dr(
    summary: &mut PionTruthDiagnosisSummary,
    plus: &BTreeMap<String, Option<f64>>,
    minus: &BTreeMap<String, Option<f64>>,
) {
    push_nearest(
        summary,
        "nearest_pi_plus_dr",
        plus.get("pi_only_same_charge"),
    );
    push_nearest(
        summary,
        "nearest_pi_minus_dr",
        minus.get("pi_only_same_charge"),
    );
    push_nearest(
        summary,
        "nearest_any_charged_hadron_plus_dr",
        plus.get("any_charged_hadron_same_charge"),
    );
    push_nearest(
        summary,
        "nearest_any_charged_hadron_minus_dr",
        minus.get("any_charged_hadron_same_charge"),
    );
    push_nearest(
        summary,
        "nearest_any_charged_stable_like_plus_dr",
        plus.get("any_charged_stable_like_same_charge"),
    );
    push_nearest(
        summary,
        "nearest_any_charged_stable_like_minus_dr",
        minus.get("any_charged_stable_like_same_charge"),
    );
}

fn push_nearest(summary: &mut PionTruthDiagnosisSummary, name: &str, value: Option<&Option<f64>>) {
    if let Some(Some(value)) = value {
        summary
            .nearest_dr
            .entry(name.to_string())
            .or_default()
            .push(*value);
    }
}

fn classify_candidate(
    candidate: &RecoCandidate,
    plus: &BTreeMap<String, Option<f64>>,
    minus: &BTreeMap<String, Option<f64>>,
) -> Vec<String> {
    let plus_pi = matched_below(plus, "pi_only_same_charge", 0.10);
    let minus_pi = matched_below(minus, "pi_only_same_charge", 0.10);
    let both_hadron = matched_below(plus, "any_charged_hadron_same_charge", 0.10)
        && matched_below(minus, "any_charged_hadron_same_charge", 0.10);
    let both_stable = matched_below(plus, "any_charged_stable_like_same_charge", 0.10)
        && matched_below(minus, "any_charged_stable_like_same_charge", 0.10);

    let mut categories = Vec::new();
    match (plus_pi, minus_pi) {
        (true, true) => categories.push("both_reco_pions_match_gen_pions_dr0p1"),
        (true, false) => categories.push("only_plus_matches_gen_pion_dr0p1"),
        (false, true) => categories.push("only_minus_matches_gen_pion_dr0p1"),
        (false, false) => categories.push("no_gen_pion_match_dr0p1"),
    }
    if both_hadron {
        categories.push("both_match_any_charged_hadron_dr0p1");
    }
    if both_stable {
        categories.push("both_match_any_charged_stable_like_dr0p1");
    }
    if candidate.photon_matched && !(plus_pi && minus_pi) {
        categories.push("photon_matched_but_no_pion_match");
    }
    categories.into_iter().map(str::to_string).collect()
}

fn matched_below(matches: &BTreeMap<String, Option<f64>>, pool: &str, threshold: f64) -> bool {
    matches
        .get(pool)
        .and_then(|value| *value)
        .is_some_and(|value| value < threshold)
}

pub fn recommendation(summary: &PionTruthDiagnosisSummary) -> &'static str {
    let total = summary.diagnosed_candidates.max(1);
    let pion = summary
        .both_threshold_counts
        .get("pi_only_same_charge")
        .and_then(|pool| pool.get("0.10"))
        .copied()
        .unwrap_or(0);
    let hadron = summary
        .both_threshold_counts
        .get("any_charged_hadron_same_charge")
        .and_then(|pool| pool.get("0.10"))
        .copied()
        .unwrap_or(0);
    let events_with_pi = summary
        .pool_event_counts
        .get("all_pi_plus")
        .copied()
        .unwrap_or(0)
        .min(
            summary
                .pool_event_counts
                .get("all_pi_minus")
                .copied()
                .unwrap_or(0),
        );

    if events_with_pi * 10 < total {
        "Use photon truth only until PackedGenPart or tracking truth can provide charged-pion truth."
    } else if hadron > pion * 5 && hadron > 0 {
        "Charged-hadron matching works better than pion-only matching; treat rho as a charged-candidate mass hypothesis."
    } else if pion == 0 {
        "GenPart charged particles are far from accepted reco pions; survey PackedGenPart or tracking truth next."
    } else {
        "GenPart pions exist but matching is sparse; use this as a diagnostic only before any cut study."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn particle(pdg_id: i32, eta: f64, phi: f64) -> GenPartRecord {
        GenPartRecord {
            pdg_id,
            mother: None,
            status: 1,
            status_flags: 0,
            pt: 10.0,
            eta,
            phi,
            mass: 0.139,
        }
    }

    fn candidate() -> RecoCandidate {
        RecoCandidate {
            run: 1,
            luminosity_block: 2,
            event: 3,
            pi_plus_pt: 12.0,
            pi_plus_eta: 0.0,
            pi_plus_phi: 0.0,
            pi_minus_pt: 8.0,
            pi_minus_eta: 1.0,
            pi_minus_phi: 1.0,
            h_mass: 125.0,
            rho_mass: 0.77,
            rho_pt_over_photon_pt: 0.4,
            photon_matched: true,
        }
    }

    #[test]
    fn nearest_same_charge_pions_match_thresholds() {
        let particles = vec![particle(211, 0.01, 0.0), particle(-211, 1.02, 1.0)];
        let mut summary = PionTruthDiagnosisSummary::new(1);

        summary.add_event_candidates(&[candidate()], &particles);

        assert_eq!(summary.diagnosed_candidates, 1);
        assert_eq!(
            summary.category_counts["both_reco_pions_match_gen_pions_dr0p1"],
            1
        );
        assert_eq!(
            summary.both_threshold_counts["pi_only_same_charge"]["0.10"],
            1
        );
        assert_eq!(summary.nearest_dr["nearest_pi_plus_dr"].len(), 1);
    }

    #[test]
    fn charged_hadron_pool_can_match_when_pion_pool_does_not() {
        let particles = vec![particle(321, 0.01, 0.0), particle(-2212, 1.02, 1.0)];
        let mut summary = PionTruthDiagnosisSummary::new(1);

        summary.add_event_candidates(&[candidate()], &particles);

        assert_eq!(summary.category_counts["no_gen_pion_match_dr0p1"], 1);
        assert_eq!(
            summary.category_counts["both_match_any_charged_hadron_dr0p1"],
            1
        );
        assert_eq!(
            summary.both_threshold_counts["any_charged_hadron_same_charge"]["0.10"],
            1
        );
    }

    #[test]
    fn invalid_fake_particle_kinematics_are_ignored() {
        let mut bad = particle(211, f64::NAN, 0.0);
        bad.pt = 10.0;
        let particles = vec![bad, particle(-211, 1.02, 1.0)];
        let mut summary = PionTruthDiagnosisSummary::new(1);

        summary.add_event_candidates(&[candidate()], &particles);

        assert_eq!(
            summary.leg_threshold_counts["pi_plus_leg"]["pi_only_same_charge"]["0.50"],
            0
        );
        assert_eq!(
            summary.category_counts["only_minus_matches_gen_pion_dr0p1"],
            1
        );
    }

    #[test]
    fn charged_lepton_charge_sign_follows_pdg_convention() {
        assert_eq!(pdg_charge_sign(-11), Some(1));
        assert_eq!(pdg_charge_sign(11), Some(-1));
        assert!(same_charge(1, -13));
        assert!(!same_charge(1, 13));
    }
}
