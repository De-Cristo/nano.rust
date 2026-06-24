use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HToRhoGammaCuts {
    pub photon_min_pt: f64,
    pub pi1_min_pt: f64,
    pub pi2_min_pt: f64,
    pub max_delta_r_pipi: f64,
    pub rho_mass_min: f64,
    pub rho_mass_max: f64,
    pub min_delta_r_gamma_rho: f64,
    pub max_delta_r_gamma_rho: f64,
    pub pion_mass: f64,
    pub rho_mass_target: f64,
    pub higgs_mass_reference: f64,
}

impl HToRhoGammaCuts {
    pub fn zcountinghlt_naive() -> Self {
        Self {
            photon_min_pt: 15.0,
            pi1_min_pt: 5.0,
            pi2_min_pt: 2.0,
            max_delta_r_pipi: 0.1,
            rho_mass_min: 0.3,
            rho_mass_max: 1.2,
            min_delta_r_gamma_rho: 1.0,
            max_delta_r_gamma_rho: 5.0,
            pion_mass: 0.139_570_39,
            rho_mass_target: 0.775_26,
            higgs_mass_reference: 125.0,
        }
    }
}

impl Default for HToRhoGammaCuts {
    fn default() -> Self {
        Self::zcountinghlt_naive()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleCand {
    pub pt: f64,
    pub eta: f64,
    pub phi: f64,
    pub mass: f64,
    pub charge: i32,
    pub p4: FourVec,
}

impl SimpleCand {
    pub fn new(pt: f64, eta: f64, phi: f64, mass: f64, charge: i32) -> Self {
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

#[derive(Debug, Clone, PartialEq)]
pub struct RhoCand {
    pub pi_plus: SimpleCand,
    pub pi_minus: SimpleCand,
    pub p4: FourVec,
    pub mass: f64,
    pub pt: f64,
    pub eta: f64,
    pub phi: f64,
    pub pipi_delta_r: f64,
}

impl RhoCand {
    pub fn from_pair(first: SimpleCand, second: SimpleCand, pipi_delta_r: f64) -> Self {
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

#[derive(Debug, Clone, PartialEq)]
pub struct HCand {
    pub gamma: SimpleCand,
    pub rho: RhoCand,
    pub p4: FourVec,
    pub mass: f64,
    pub pt: f64,
    pub eta: f64,
    pub phi: f64,
}

impl HCand {
    pub fn new(gamma: SimpleCand, rho: RhoCand) -> Self {
        let p4 = gamma.p4.add(&rho.p4);
        Self {
            gamma,
            rho,
            p4,
            mass: p4.mass(),
            pt: p4.pt(),
            eta: p4.eta(),
            phi: p4.phi(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FourVec {
    pub px: f64,
    pub py: f64,
    pub pz: f64,
    pub e: f64,
}

impl FourVec {
    pub fn from_pt_eta_phi_mass(pt: f64, eta: f64, phi: f64, mass: f64) -> Self {
        let px = pt * phi.cos();
        let py = pt * phi.sin();
        let pz = pt * eta.sinh();
        let p2 = px * px + py * py + pz * pz;
        let e = (p2 + mass * mass).sqrt();
        Self { px, py, pz, e }
    }

    pub fn add(&self, other: &Self) -> Self {
        Self {
            px: self.px + other.px,
            py: self.py + other.py,
            pz: self.pz + other.pz,
            e: self.e + other.e,
        }
    }

    pub fn pt(&self) -> f64 {
        self.px.hypot(self.py)
    }

    pub fn eta(&self) -> f64 {
        let pt = self.pt();
        if pt > 0.0 {
            (self.pz / pt).asinh()
        } else {
            0.0
        }
    }

    pub fn phi(&self) -> f64 {
        self.py.atan2(self.px)
    }

    pub fn mass(&self) -> f64 {
        let p2 = self.px * self.px + self.py * self.py + self.pz * self.pz;
        (self.e * self.e - p2).max(0.0).sqrt()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EventInputs<'a> {
    pub photon_pt: &'a [f32],
    pub photon_eta: &'a [f32],
    pub photon_phi: &'a [f32],
    pub pfcand_pt: &'a [f32],
    pub pfcand_eta: &'a [f32],
    pub pfcand_phi: &'a [f32],
    pub pfcand_pdg_id: &'a [i32],
    pub pfcand_mass: Option<&'a [f32]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventRecoResult {
    pub photon: Option<SimpleCand>,
    pub pions: Vec<SimpleCand>,
    pub plus_pion_count: usize,
    pub minus_pion_count: usize,
    pub source_pion_mass_sum: f64,
    pub source_pion_mass_count: usize,
    pub has_os_pt_pair: bool,
    pub has_pipi_delta_r_pair: bool,
    pub rho: Option<RhoCand>,
    pub gamma_rho_delta_r: Option<f64>,
    pub h: Option<HCand>,
}

impl EventRecoResult {
    pub fn average_source_pion_mass(&self) -> Option<f64> {
        (self.source_pion_mass_count > 0)
            .then_some(self.source_pion_mass_sum / self.source_pion_mass_count as f64)
    }
}

pub fn reconstruct_event(inputs: EventInputs<'_>, cuts: &HToRhoGammaCuts) -> EventRecoResult {
    let photon =
        select_leading_photon(inputs.photon_pt, inputs.photon_eta, inputs.photon_phi, cuts);
    let PionSelection {
        mut pions,
        plus_pion_count,
        minus_pion_count,
        source_pion_mass_sum,
        source_pion_mass_count,
    } = collect_pions(inputs, cuts);

    pions.sort_by(|a, b| b.pt.partial_cmp(&a.pt).unwrap_or(Ordering::Equal));
    let PairSearch {
        has_os_pt_pair,
        has_pipi_delta_r_pair,
        rho,
    } = find_best_rho(&pions, cuts);

    let gamma_rho_delta_r = photon
        .as_ref()
        .zip(rho.as_ref())
        .map(|(photon, rho)| delta_r(photon.eta, photon.phi, rho.eta, rho.phi));
    let h = photon
        .clone()
        .zip(rho.clone())
        .zip(gamma_rho_delta_r)
        .and_then(|((photon, rho), gamma_rho_delta_r)| {
            (cuts.min_delta_r_gamma_rho..=cuts.max_delta_r_gamma_rho)
                .contains(&gamma_rho_delta_r)
                .then(|| HCand::new(photon, rho))
        });

    EventRecoResult {
        photon,
        pions,
        plus_pion_count,
        minus_pion_count,
        source_pion_mass_sum,
        source_pion_mass_count,
        has_os_pt_pair,
        has_pipi_delta_r_pair,
        rho,
        gamma_rho_delta_r,
        h,
    }
}

pub fn delta_r(eta_a: f64, phi_a: f64, eta_b: f64, phi_b: f64) -> f64 {
    let deta = eta_a - eta_b;
    let dphi = delta_phi(phi_a, phi_b);
    deta.hypot(dphi)
}

pub fn delta_phi(phi_a: f64, phi_b: f64) -> f64 {
    let mut dphi = phi_a - phi_b;
    while dphi > std::f64::consts::PI {
        dphi -= 2.0 * std::f64::consts::PI;
    }
    while dphi <= -std::f64::consts::PI {
        dphi += 2.0 * std::f64::consts::PI;
    }
    dphi
}

fn select_leading_photon(
    pt: &[f32],
    eta: &[f32],
    phi: &[f32],
    cuts: &HToRhoGammaCuts,
) -> Option<SimpleCand> {
    pt.iter()
        .zip(eta)
        .zip(phi)
        .filter_map(|((&pt, &eta), &phi)| {
            let pt = f64::from(pt);
            (pt >= cuts.photon_min_pt)
                .then(|| SimpleCand::new(pt, f64::from(eta), f64::from(phi), 0.0, 0))
        })
        .max_by(|a, b| a.pt.partial_cmp(&b.pt).unwrap_or(Ordering::Equal))
}

struct PionSelection {
    pions: Vec<SimpleCand>,
    plus_pion_count: usize,
    minus_pion_count: usize,
    source_pion_mass_sum: f64,
    source_pion_mass_count: usize,
}

fn collect_pions(inputs: EventInputs<'_>, cuts: &HToRhoGammaCuts) -> PionSelection {
    let mut pions = Vec::new();
    let mut plus_pion_count = 0;
    let mut minus_pion_count = 0;
    let mut source_pion_mass_sum = 0.0;
    let mut source_pion_mass_count = 0;

    for index in 0..inputs.pfcand_pdg_id.len() {
        match inputs.pfcand_pdg_id[index] {
            211 => plus_pion_count += 1,
            -211 => minus_pion_count += 1,
            _ => {}
        }

        if inputs.pfcand_pdg_id[index].abs() != 211 {
            continue;
        }
        let charge = inputs.pfcand_pdg_id[index].signum();
        if charge == 0 {
            continue;
        }
        let pt = f64::from(inputs.pfcand_pt[index]);
        if pt < cuts.pi2_min_pt {
            continue;
        }
        if let Some(mass) = inputs.pfcand_mass {
            source_pion_mass_sum += f64::from(mass[index]);
            source_pion_mass_count += 1;
        }
        pions.push(SimpleCand::new(
            pt,
            f64::from(inputs.pfcand_eta[index]),
            f64::from(inputs.pfcand_phi[index]),
            cuts.pion_mass,
            charge,
        ));
    }

    PionSelection {
        pions,
        plus_pion_count,
        minus_pion_count,
        source_pion_mass_sum,
        source_pion_mass_count,
    }
}

#[derive(Debug)]
struct PairSearch {
    has_os_pt_pair: bool,
    has_pipi_delta_r_pair: bool,
    rho: Option<RhoCand>,
}

fn find_best_rho(pions: &[SimpleCand], cuts: &HToRhoGammaCuts) -> PairSearch {
    let mut has_os_pt_pair = false;
    let mut has_pipi_delta_r_pair = false;
    let mut best: Option<(f64, RhoCand)> = None;

    for i in 0..pions.len() {
        for j in (i + 1)..pions.len() {
            let leading = &pions[i];
            let subleading = &pions[j];
            if leading.pt < cuts.pi1_min_pt || subleading.pt < cuts.pi2_min_pt {
                continue;
            }
            if leading.charge * subleading.charge >= 0 {
                continue;
            }
            has_os_pt_pair = true;

            let pipi_delta_r = delta_r(leading.eta, leading.phi, subleading.eta, subleading.phi);
            if pipi_delta_r >= cuts.max_delta_r_pipi {
                continue;
            }
            has_pipi_delta_r_pair = true;

            let rho = RhoCand::from_pair(leading.clone(), subleading.clone(), pipi_delta_r);
            if !(cuts.rho_mass_min..=cuts.rho_mass_max).contains(&rho.mass) {
                continue;
            }

            let distance = (rho.mass - cuts.rho_mass_target).abs();
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
        has_pipi_delta_r_pair,
        rho: best.map(|(_, rho)| rho),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cuts() -> HToRhoGammaCuts {
        HToRhoGammaCuts::zcountinghlt_naive()
    }

    #[test]
    fn delta_phi_wraps_across_pi_boundary() {
        let wrapped = delta_phi(3.10, -3.10);
        assert!((wrapped + 0.08318530717958605).abs() < 1e-12);
    }

    #[test]
    fn four_vec_reconstructs_input_mass() {
        let p4 = FourVec::from_pt_eta_phi_mass(45.0, 0.8, -1.3, 0.139_570_39);
        assert!((p4.mass() - 0.139_570_39).abs() < 1e-8);
    }

    #[test]
    fn selects_highest_pt_photon_above_threshold() {
        let reco = reconstruct_event(
            EventInputs {
                photon_pt: &[10.0, 16.0, 25.0],
                photon_eta: &[0.0, 0.2, -0.3],
                photon_phi: &[0.0, 0.4, -0.5],
                pfcand_pt: &[],
                pfcand_eta: &[],
                pfcand_phi: &[],
                pfcand_pdg_id: &[],
                pfcand_mass: None,
            },
            &default_cuts(),
        );

        let photon = reco.photon.expect("selected photon");
        assert_eq!(photon.pt, 25.0);
        assert_eq!(photon.charge, 0);
    }

    #[test]
    fn selects_pions_from_signed_pdg_id_and_rejects_non_pions() {
        let reco = reconstruct_event(
            EventInputs {
                photon_pt: &[20.0],
                photon_eta: &[0.0],
                photon_phi: &[0.0],
                pfcand_pt: &[8.0, 7.0, 9.0, 1.0],
                pfcand_eta: &[0.0, 0.01, 0.02, 0.03],
                pfcand_phi: &[2.0, 2.02, 2.03, 2.04],
                pfcand_pdg_id: &[211, -211, 13, 211],
                pfcand_mass: Some(&[0.139, 0.140, 0.105, 0.139]),
            },
            &default_cuts(),
        );

        assert_eq!(reco.plus_pion_count, 2);
        assert_eq!(reco.minus_pion_count, 1);
        assert_eq!(reco.pions.len(), 2);
        assert_eq!(reco.pions[0].charge, 1);
        assert_eq!(reco.pions[1].charge, -1);
    }

    #[test]
    fn requires_opposite_sign_pion_pair() {
        let reco = reconstruct_event(
            EventInputs {
                photon_pt: &[30.0],
                photon_eta: &[0.0],
                photon_phi: &[0.0],
                pfcand_pt: &[20.0, 10.0],
                pfcand_eta: &[0.0, 0.01],
                pfcand_phi: &[2.0, 2.01],
                pfcand_pdg_id: &[211, 211],
                pfcand_mass: None,
            },
            &default_cuts(),
        );

        assert!(!reco.has_os_pt_pair);
        assert!(reco.h.is_none());
    }

    #[test]
    fn chooses_best_rho_pair_closest_to_target() {
        let cuts = default_cuts();
        let reco = reconstruct_event(
            EventInputs {
                photon_pt: &[80.0],
                photon_eta: &[0.0],
                photon_phi: &[0.0],
                pfcand_pt: &[30.0, 20.0, 12.0],
                pfcand_eta: &[0.0, 0.0, 0.0],
                pfcand_phi: &[2.20, 2.228, 2.25],
                pfcand_pdg_id: &[211, -211, -211],
                pfcand_mass: None,
            },
            &cuts,
        );

        let rho = reco.rho.expect("rho");
        assert!((rho.mass - cuts.rho_mass_target).abs() < 0.10);
        assert!((rho.pi_minus.pt - 20.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_gamma_rho_delta_r_outside_window() {
        let mut cuts = default_cuts();
        cuts.min_delta_r_gamma_rho = 1.0;
        cuts.max_delta_r_gamma_rho = 2.0;

        let reco = reconstruct_event(
            EventInputs {
                photon_pt: &[40.0],
                photon_eta: &[0.0],
                photon_phi: &[0.0],
                pfcand_pt: &[20.0, 10.0],
                pfcand_eta: &[0.0, 0.0],
                pfcand_phi: &[0.020, 0.050],
                pfcand_pdg_id: &[211, -211],
                pfcand_mass: None,
            },
            &cuts,
        );

        assert!(reco.rho.is_some());
        assert!(reco.gamma_rho_delta_r.is_some());
        assert!(reco.h.is_none());
    }

    #[test]
    fn reconstructs_minimal_synthetic_h_candidate() {
        let reco = reconstruct_event(
            EventInputs {
                photon_pt: &[100.0],
                photon_eta: &[0.0],
                photon_phi: &[0.0],
                pfcand_pt: &[20.0, 10.0],
                pfcand_eta: &[0.0, 0.0],
                pfcand_phi: &[2.020, 2.050],
                pfcand_pdg_id: &[211, -211],
                pfcand_mass: None,
            },
            &default_cuts(),
        );

        assert!(reco.has_os_pt_pair);
        assert!(reco.has_pipi_delta_r_pair);
        assert!(reco.rho.is_some());
        assert!(reco.h.is_some());
    }
}
