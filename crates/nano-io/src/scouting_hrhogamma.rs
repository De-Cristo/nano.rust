use std::cmp::Ordering;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

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

    pub fn from_config_path(path: &Path) -> Result<Self, HToRhoGammaConfigError> {
        let contents = std::fs::read_to_string(path).map_err(HToRhoGammaConfigError::Io)?;
        Self::from_config_toml_str(&contents)
    }

    pub fn from_config_toml_str(contents: &str) -> Result<Self, HToRhoGammaConfigError> {
        let config: HToRhoGammaConfig =
            toml::from_str(contents).map_err(HToRhoGammaConfigError::Parse)?;
        let cuts = config.baseline.zcountinghlt_naive.into_cuts();
        cuts.validate()?;
        Ok(cuts)
    }

    pub fn validate(&self) -> Result<(), HToRhoGammaConfigError> {
        if self.photon_min_pt < 0.0 {
            return Err(HToRhoGammaConfigError::Validation(
                "photon_min_pt must be >= 0".to_string(),
            ));
        }
        if self.pi2_min_pt < 0.0 {
            return Err(HToRhoGammaConfigError::Validation(
                "pi2_min_pt must be >= 0".to_string(),
            ));
        }
        if self.pi1_min_pt < self.pi2_min_pt {
            return Err(HToRhoGammaConfigError::Validation(
                "pi1_min_pt must be >= pi2_min_pt".to_string(),
            ));
        }
        if self.max_delta_r_pipi <= 0.0 {
            return Err(HToRhoGammaConfigError::Validation(
                "max_delta_r_pipi must be > 0".to_string(),
            ));
        }
        if self.rho_mass_min >= self.rho_mass_max {
            return Err(HToRhoGammaConfigError::Validation(
                "rho_mass_min must be < rho_mass_max".to_string(),
            ));
        }
        if self.min_delta_r_gamma_rho > self.max_delta_r_gamma_rho {
            return Err(HToRhoGammaConfigError::Validation(
                "min_delta_r_gamma_rho must be <= max_delta_r_gamma_rho".to_string(),
            ));
        }
        if self.pion_mass <= 0.0 {
            return Err(HToRhoGammaConfigError::Validation(
                "pion_mass must be > 0".to_string(),
            ));
        }
        if self.rho_mass_target <= 0.0 {
            return Err(HToRhoGammaConfigError::Validation(
                "rho_mass_target must be > 0".to_string(),
            ));
        }
        if self.higgs_mass_reference <= 0.0 {
            return Err(HToRhoGammaConfigError::Validation(
                "higgs_mass_reference must be > 0".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for HToRhoGammaCuts {
    fn default() -> Self {
        Self::zcountinghlt_naive()
    }
}

#[derive(Debug)]
pub enum HToRhoGammaConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Validation(String),
}

impl fmt::Display for HToRhoGammaConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to read config: {err}"),
            Self::Parse(err) => write!(f, "failed to parse TOML: {err}"),
            Self::Validation(message) => write!(f, "invalid HToRhoGamma cuts: {message}"),
        }
    }
}

impl std::error::Error for HToRhoGammaConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Parse(err) => Some(err),
            Self::Validation(_) => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct HToRhoGammaConfig {
    analysis: Option<AnalysisConfig>,
    objects: Option<ObjectsConfig>,
    baseline: BaselineConfig,
}

#[derive(Debug, Deserialize)]
struct AnalysisConfig {
    branch_catalogue: String,
}

#[derive(Debug, Deserialize)]
struct ObjectsConfig {
    photon: ObjectSourceConfig,
    charged_candidate: ObjectSourceConfig,
}

#[derive(Debug, Deserialize)]
struct ObjectSourceConfig {
    source: String,
}

#[derive(Debug, Deserialize)]
struct BaselineConfig {
    zcountinghlt_naive: HToRhoGammaCutsConfig,
}

#[derive(Debug, Deserialize)]
struct HToRhoGammaCutsConfig {
    photon_min_pt: f64,
    pi1_min_pt: f64,
    pi2_min_pt: f64,
    max_delta_r_pipi: f64,
    rho_mass_min: f64,
    rho_mass_max: f64,
    min_delta_r_gamma_rho: f64,
    max_delta_r_gamma_rho: f64,
    pion_mass: f64,
    rho_mass_target: f64,
    higgs_mass_reference: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HToRhoGammaBranchMapping {
    pub photon: PhotonBranches,
    pub charged_candidate: ChargedCandidateBranches,
}

impl HToRhoGammaBranchMapping {
    pub fn zcountinghlt_naive() -> Self {
        Self {
            photon: PhotonBranches {
                semantic_name: "ScoutingPhoton".to_string(),
                count: "nPhoton".to_string(),
                pt: "Photon_pt".to_string(),
                eta: "Photon_eta".to_string(),
                phi: "Photon_phi".to_string(),
            },
            charged_candidate: ChargedCandidateBranches {
                semantic_name: "ScoutingChargedCandidate".to_string(),
                count: "nPFCand".to_string(),
                pt: "PFCand_pt".to_string(),
                eta: "PFCand_eta".to_string(),
                phi: "PFCand_phi".to_string(),
                pdg_id: "PFCand_pdgId".to_string(),
                mass: Some("PFCand_mass".to_string()),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhotonBranches {
    pub semantic_name: String,
    pub count: String,
    pub pt: String,
    pub eta: String,
    pub phi: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChargedCandidateBranches {
    pub semantic_name: String,
    pub count: String,
    pub pt: String,
    pub eta: String,
    pub phi: String,
    pub pdg_id: String,
    pub mass: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BranchCatalogue {
    objects: std::collections::HashMap<String, CatalogueObject>,
}

#[derive(Debug, Deserialize)]
struct CatalogueObject {
    count: String,
    fields: std::collections::HashMap<String, String>,
}

pub fn load_branch_mapping(
    config_path: &Path,
) -> Result<(String, HToRhoGammaBranchMapping), Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(config_path)?;
    let config: HToRhoGammaConfig = toml::from_str(&contents)?;

    let analysis = config.analysis.ok_or("missing [analysis] table")?;
    let objects = config.objects.ok_or("missing [objects] table")?;

    let cat_rel_path = analysis.branch_catalogue;
    let mut catalogue_path = config_path
        .parent()
        .unwrap_or(Path::new(""))
        .join(&cat_rel_path);
    if !catalogue_path.exists() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap_or(Path::new(""));
        catalogue_path = workspace_root.join(&cat_rel_path);
    }

    if !catalogue_path.exists() {
        return Err(format!("branch_catalogue not found: {}", catalogue_path.display()).into());
    }

    let cat_contents = std::fs::read_to_string(&catalogue_path)?;
    let catalogue: BranchCatalogue = serde_yaml::from_str(&cat_contents)?;

    let pho_src = &objects.photon.source;
    let pho_obj = catalogue
        .objects
        .get(pho_src)
        .ok_or_else(|| format!("missing photon object: {}", pho_src))?;
    let photon = PhotonBranches {
        semantic_name: pho_src.clone(),
        count: pho_obj.count.clone(),
        pt: pho_obj.fields.get("pt").ok_or("missing pt")?.clone(),
        eta: pho_obj.fields.get("eta").ok_or("missing eta")?.clone(),
        phi: pho_obj.fields.get("phi").ok_or("missing phi")?.clone(),
    };

    let chg_src = &objects.charged_candidate.source;
    let chg_obj = catalogue
        .objects
        .get(chg_src)
        .ok_or_else(|| format!("missing charged_candidate object: {}", chg_src))?;
    let charged_candidate = ChargedCandidateBranches {
        semantic_name: chg_src.clone(),
        count: chg_obj.count.clone(),
        pt: chg_obj.fields.get("pt").ok_or("missing pt")?.clone(),
        eta: chg_obj.fields.get("eta").ok_or("missing eta")?.clone(),
        phi: chg_obj.fields.get("phi").ok_or("missing phi")?.clone(),
        pdg_id: chg_obj.fields.get("pdgId").ok_or("missing pdgId")?.clone(),
        mass: chg_obj.fields.get("mass").cloned(),
    };

    Ok((
        cat_rel_path,
        HToRhoGammaBranchMapping {
            photon,
            charged_candidate,
        },
    ))
}

impl HToRhoGammaCutsConfig {
    fn into_cuts(self) -> HToRhoGammaCuts {
        HToRhoGammaCuts {
            photon_min_pt: self.photon_min_pt,
            pi1_min_pt: self.pi1_min_pt,
            pi2_min_pt: self.pi2_min_pt,
            max_delta_r_pipi: self.max_delta_r_pipi,
            rho_mass_min: self.rho_mass_min,
            rho_mass_max: self.rho_mass_max,
            min_delta_r_gamma_rho: self.min_delta_r_gamma_rho,
            max_delta_r_gamma_rho: self.max_delta_r_gamma_rho,
            pion_mass: self.pion_mass,
            rho_mass_target: self.rho_mass_target,
            higgs_mass_reference: self.higgs_mass_reference,
        }
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

    pub fn sub(&self, other: &Self) -> Self {
        Self {
            px: self.px - other.px,
            py: self.py - other.py,
            pz: self.pz - other.pz,
            e: self.e - other.e,
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

pub const TRUTH_MATCH_DELTA_R_MAX: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GenParticle {
    pub pdg_id: i32,
    pub mother_index: Option<usize>,
    pub pt: f64,
    pub eta: f64,
    pub phi: f64,
    pub mass: f64,
    pub status: i32,
    pub status_flags: u16,
}

impl GenParticle {
    pub fn new(
        pdg_id: i32,
        mother_index: Option<usize>,
        pt: f64,
        eta: f64,
        phi: f64,
        mass: f64,
    ) -> Self {
        Self {
            pdg_id,
            mother_index,
            pt,
            eta,
            phi,
            mass,
            status: 0,
            status_flags: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_status(
        pdg_id: i32,
        mother_index: Option<usize>,
        pt: f64,
        eta: f64,
        phi: f64,
        mass: f64,
        status: i32,
        status_flags: u16,
    ) -> Self {
        Self {
            pdg_id,
            mother_index,
            pt,
            eta,
            phi,
            mass,
            status,
            status_flags,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruthStrategy {
    None,
    ExplicitChain,
    TopologyProxy,
    HgammaClosure,
}

impl TruthStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ExplicitChain => "explicit_chain",
            Self::TopologyProxy => "topology_proxy",
            Self::HgammaClosure => "hgamma_closure",
        }
    }

    pub fn code(self) -> i32 {
        match self {
            Self::None => 0,
            Self::TopologyProxy => 1,
            Self::HgammaClosure => 2,
            Self::ExplicitChain => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruthTopology {
    NotAvailable,
    ExplicitRho,
    FallbackNoExplicitRho,
    NotFound,
}

impl TruthTopology {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotAvailable => "not_available",
            Self::ExplicitRho => "explicit_rho",
            Self::FallbackNoExplicitRho => "fallback_no_explicit_rho",
            Self::NotFound => "not_found",
        }
    }

    pub fn code(self) -> i32 {
        match self {
            Self::NotAvailable => 0,
            Self::ExplicitRho => 1,
            Self::FallbackNoExplicitRho => 2,
            Self::NotFound => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HToRhoGammaTruth {
    pub topology: TruthTopology,
    pub h: Option<GenParticle>,
    pub rho: Option<GenParticle>,
    pub photon: Option<GenParticle>,
    pub pi_plus: Option<GenParticle>,
    pub pi_minus: Option<GenParticle>,
}

impl HToRhoGammaTruth {
    pub fn not_available() -> Self {
        Self {
            topology: TruthTopology::NotAvailable,
            h: None,
            rho: None,
            photon: None,
            pi_plus: None,
            pi_minus: None,
        }
    }

    pub fn not_found(h: Option<GenParticle>) -> Self {
        Self {
            topology: TruthTopology::NotFound,
            h,
            rho: None,
            photon: None,
            pi_plus: None,
            pi_minus: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TruthMatchResult {
    pub truth_strategy: TruthStrategy,
    pub truth_available: bool,
    pub truth_topology: TruthTopology,
    pub truth_matched: bool,
    pub truth_proxy_matched: bool,
    pub truth_proxy_matched_dr_0p1: bool,
    pub truth_proxy_matched_dr_0p2: bool,
    pub truth_proxy_matched_dr_0p3: bool,
    pub truth_photon_anchor_available: bool,
    pub gen_photon_from_higgs: bool,
    pub nearest_gen_pi_plus_available: bool,
    pub nearest_gen_pi_minus_available: bool,
    pub gen_h: Option<GenParticle>,
    pub gen_rho: Option<GenParticle>,
    pub gen_photon: Option<GenParticle>,
    pub gen_pi_plus: Option<GenParticle>,
    pub gen_pi_minus: Option<GenParticle>,
    pub gen_h_proxy: Option<GenParticle>,
    pub gen_rho_proxy: Option<GenParticle>,
    pub delta_r_reco_photon_gen_photon: Option<f64>,
    pub delta_r_reco_pi_plus_gen_pi_plus: Option<f64>,
    pub delta_r_reco_pi_minus_gen_pi_minus: Option<f64>,
    pub delta_r_reco_rho_gen_rho: Option<f64>,
    pub delta_r_reco_rho_gen_rho_proxy: Option<f64>,
    pub delta_r_reco_h_gen_h_proxy: Option<f64>,
    pub reco_h_mass_minus_gen_h_mass: Option<f64>,
    pub reco_rho_mass_minus_gen_rho_mass: Option<f64>,
    pub reco_h_mass_minus_gen_h_proxy_mass: Option<f64>,
    pub reco_rho_mass_minus_gen_rho_proxy_mass: Option<f64>,
    pub reco_photon_pt_over_gen_photon_pt: Option<f64>,
    pub reco_rho_pt_over_gen_rho_pt: Option<f64>,
    pub reco_pi_plus_pt_over_gen_pi_plus_pt: Option<f64>,
    pub reco_pi_minus_pt_over_gen_pi_minus_pt: Option<f64>,
    pub reco_rho_pt_over_gen_rho_proxy_pt: Option<f64>,
    pub reco_h_pt_over_gen_h_proxy_pt: Option<f64>,
    pub hgamma_closure_available: bool,
    pub hgamma_closure_matched: bool,
    pub hgamma_gen_h_available: bool,
    pub hgamma_gen_gamma_available: bool,
    pub hgamma_gen_rho_recoil_available: bool,
    pub hgamma_photon_matched_dr_0p1: bool,
    pub hgamma_photon_matched_dr_0p2: bool,
    pub hgamma_higgs_closed_mass_10: bool,
    pub hgamma_higgs_closed_mass_15: bool,
    pub hgamma_higgs_closed_mass_20: bool,
    pub hgamma_higgs_closed_dr_0p3: bool,
    pub hgamma_higgs_closed_dr_0p5: bool,
    pub hgamma_gen_h: Option<GenParticle>,
    pub hgamma_gen_gamma: Option<GenParticle>,
    pub hgamma_gen_rho_recoil: Option<GenParticle>,
    pub reco_photon_eta_minus_gen_photon_eta: Option<f64>,
    pub reco_photon_phi_minus_gen_photon_phi: Option<f64>,
    pub delta_r_reco_h_gen_h: Option<f64>,
    pub reco_h_pt_over_gen_h_pt: Option<f64>,
    pub delta_r_reco_rho_gen_rho_recoil: Option<f64>,
    pub reco_rho_mass_minus_gen_rho_recoil_mass: Option<f64>,
    pub reco_rho_pt_over_gen_rho_recoil_pt: Option<f64>,
}

impl TruthMatchResult {
    pub fn not_available() -> Self {
        Self {
            truth_strategy: TruthStrategy::None,
            truth_available: false,
            truth_topology: TruthTopology::NotAvailable,
            truth_matched: false,
            truth_proxy_matched: false,
            truth_proxy_matched_dr_0p1: false,
            truth_proxy_matched_dr_0p2: false,
            truth_proxy_matched_dr_0p3: false,
            truth_photon_anchor_available: false,
            gen_photon_from_higgs: false,
            nearest_gen_pi_plus_available: false,
            nearest_gen_pi_minus_available: false,
            gen_h: None,
            gen_rho: None,
            gen_photon: None,
            gen_pi_plus: None,
            gen_pi_minus: None,
            gen_h_proxy: None,
            gen_rho_proxy: None,
            delta_r_reco_photon_gen_photon: None,
            delta_r_reco_pi_plus_gen_pi_plus: None,
            delta_r_reco_pi_minus_gen_pi_minus: None,
            delta_r_reco_rho_gen_rho: None,
            delta_r_reco_rho_gen_rho_proxy: None,
            delta_r_reco_h_gen_h_proxy: None,
            reco_h_mass_minus_gen_h_mass: None,
            reco_rho_mass_minus_gen_rho_mass: None,
            reco_h_mass_minus_gen_h_proxy_mass: None,
            reco_rho_mass_minus_gen_rho_proxy_mass: None,
            reco_photon_pt_over_gen_photon_pt: None,
            reco_rho_pt_over_gen_rho_pt: None,
            reco_pi_plus_pt_over_gen_pi_plus_pt: None,
            reco_pi_minus_pt_over_gen_pi_minus_pt: None,
            reco_rho_pt_over_gen_rho_proxy_pt: None,
            reco_h_pt_over_gen_h_proxy_pt: None,
            hgamma_closure_available: false,
            hgamma_closure_matched: false,
            hgamma_gen_h_available: false,
            hgamma_gen_gamma_available: false,
            hgamma_gen_rho_recoil_available: false,
            hgamma_photon_matched_dr_0p1: false,
            hgamma_photon_matched_dr_0p2: false,
            hgamma_higgs_closed_mass_10: false,
            hgamma_higgs_closed_mass_15: false,
            hgamma_higgs_closed_mass_20: false,
            hgamma_higgs_closed_dr_0p3: false,
            hgamma_higgs_closed_dr_0p5: false,
            hgamma_gen_h: None,
            hgamma_gen_gamma: None,
            hgamma_gen_rho_recoil: None,
            reco_photon_eta_minus_gen_photon_eta: None,
            reco_photon_phi_minus_gen_photon_phi: None,
            delta_r_reco_h_gen_h: None,
            reco_h_pt_over_gen_h_pt: None,
            delta_r_reco_rho_gen_rho_recoil: None,
            reco_rho_mass_minus_gen_rho_recoil_mass: None,
            reco_rho_pt_over_gen_rho_recoil_pt: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HgammaClosureEventFlags {
    pub has_gen_h: bool,
    pub has_gen_hgamma: bool,
    pub has_reco_photon_preselection: bool,
    pub photon_matched_dr_0p1: bool,
    pub photon_matched_dr_0p2: bool,
    pub has_os_track_pair: bool,
    pub has_accepted_candidate: bool,
    pub higgs_closed_mass_10: bool,
    pub higgs_closed_mass_15: bool,
    pub higgs_closed_mass_20: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HgammaClosureCounters {
    pub events_total: usize,
    pub events_with_gen_h: usize,
    pub events_with_gen_hgamma: usize,
    pub events_with_reco_photon_preselection: usize,
    pub events_with_reco_photon_matched_dr_0p1: usize,
    pub events_with_reco_photon_matched_dr_0p2: usize,
    pub events_with_reco_photon_matched_dr_0p1_and_any_os_track_pair: usize,
    pub events_with_reco_photon_matched_dr_0p1_and_accepted_candidate: usize,
    pub events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_10: usize,
    pub events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_15: usize,
    pub events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_20: usize,
}

impl HgammaClosureCounters {
    pub fn observe(&mut self, flags: HgammaClosureEventFlags) {
        self.events_total += 1;
        self.events_with_gen_h += usize::from(flags.has_gen_h);
        self.events_with_gen_hgamma += usize::from(flags.has_gen_hgamma);
        self.events_with_reco_photon_preselection +=
            usize::from(flags.has_reco_photon_preselection);
        self.events_with_reco_photon_matched_dr_0p1 += usize::from(flags.photon_matched_dr_0p1);
        self.events_with_reco_photon_matched_dr_0p2 += usize::from(flags.photon_matched_dr_0p2);
        self.events_with_reco_photon_matched_dr_0p1_and_any_os_track_pair +=
            usize::from(flags.photon_matched_dr_0p1 && flags.has_os_track_pair);
        self.events_with_reco_photon_matched_dr_0p1_and_accepted_candidate +=
            usize::from(flags.photon_matched_dr_0p1 && flags.has_accepted_candidate);
        self.events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_10 +=
            usize::from(flags.photon_matched_dr_0p1 && flags.higgs_closed_mass_10);
        self.events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_15 +=
            usize::from(flags.photon_matched_dr_0p1 && flags.higgs_closed_mass_15);
        self.events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_20 +=
            usize::from(flags.photon_matched_dr_0p1 && flags.higgs_closed_mass_20);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HgammaClosureTruthObjects {
    pub gen_h: Option<GenParticle>,
    pub gen_gamma: Option<GenParticle>,
    pub gen_rho_recoil: Option<GenParticle>,
    pub delta_r_reco_photon_gen_photon: Option<f64>,
}

pub fn identify_truth_chain(particles: &[GenParticle]) -> HToRhoGammaTruth {
    let h_index = particles.iter().position(|particle| particle.pdg_id == 25);
    let Some(h_index) = h_index else {
        return HToRhoGammaTruth::not_found(None);
    };
    let h = particles[h_index];
    let photon = particles
        .iter()
        .copied()
        .find(|particle| particle.pdg_id == 22 && is_descendant_of(particles, *particle, h_index));
    let rho_index = particles.iter().position(|particle| {
        particle.pdg_id == 113 && is_descendant_of(particles, *particle, h_index)
    });
    if let Some(rho_index) = rho_index {
        let rho = particles[rho_index];
        let pi_plus = particles.iter().copied().find(|particle| {
            particle.pdg_id == 211 && is_descendant_of(particles, *particle, rho_index)
        });
        let pi_minus = particles.iter().copied().find(|particle| {
            particle.pdg_id == -211 && is_descendant_of(particles, *particle, rho_index)
        });
        if photon.is_some() && pi_plus.is_some() && pi_minus.is_some() {
            return HToRhoGammaTruth {
                topology: TruthTopology::ExplicitRho,
                h: Some(h),
                rho: Some(rho),
                photon,
                pi_plus,
                pi_minus,
            };
        }
    }

    let pi_plus = particles
        .iter()
        .copied()
        .find(|particle| particle.pdg_id == 211 && is_descendant_of(particles, *particle, h_index));
    let pi_minus = particles.iter().copied().find(|particle| {
        particle.pdg_id == -211 && is_descendant_of(particles, *particle, h_index)
    });
    if photon.is_some() && pi_plus.is_some() && pi_minus.is_some() {
        return HToRhoGammaTruth {
            topology: TruthTopology::FallbackNoExplicitRho,
            h: Some(h),
            rho: None,
            photon,
            pi_plus,
            pi_minus,
        };
    }
    HToRhoGammaTruth::not_found(Some(h))
}

fn is_descendant_of(
    particles: &[GenParticle],
    particle: GenParticle,
    ancestor_index: usize,
) -> bool {
    let mut current = particle.mother_index;
    while let Some(index) = current {
        if index == ancestor_index {
            return true;
        }
        current = particles.get(index).and_then(|mother| mother.mother_index);
    }
    false
}

pub fn match_reco_to_truth(h: &HCand, truth: Option<&HToRhoGammaTruth>) -> TruthMatchResult {
    let Some(truth) = truth else {
        return TruthMatchResult::not_available();
    };
    if matches!(truth.topology, TruthTopology::NotAvailable) {
        return TruthMatchResult::not_available();
    }

    let dr_photon = truth
        .photon
        .map(|gen| delta_r(h.gamma.eta, h.gamma.phi, gen.eta, gen.phi));
    let dr_pi_plus = truth
        .pi_plus
        .map(|gen| delta_r(h.rho.pi_plus.eta, h.rho.pi_plus.phi, gen.eta, gen.phi));
    let dr_pi_minus = truth
        .pi_minus
        .map(|gen| delta_r(h.rho.pi_minus.eta, h.rho.pi_minus.phi, gen.eta, gen.phi));
    let dr_rho = truth
        .rho
        .map(|gen| delta_r(h.rho.eta, h.rho.phi, gen.eta, gen.phi));
    let truth_matched = [dr_photon, dr_pi_plus, dr_pi_minus]
        .into_iter()
        .all(|value| value.is_some_and(|dr| dr < TRUTH_MATCH_DELTA_R_MAX));
    TruthMatchResult {
        truth_strategy: TruthStrategy::ExplicitChain,
        truth_available: !matches!(truth.topology, TruthTopology::NotAvailable),
        truth_topology: truth.topology,
        truth_matched,
        truth_proxy_matched: truth_matched,
        truth_proxy_matched_dr_0p1: truth_matched,
        truth_proxy_matched_dr_0p2: [dr_photon, dr_pi_plus, dr_pi_minus]
            .into_iter()
            .all(|value| value.is_some_and(|dr| dr < 0.2)),
        truth_proxy_matched_dr_0p3: [dr_photon, dr_pi_plus, dr_pi_minus]
            .into_iter()
            .all(|value| value.is_some_and(|dr| dr < 0.3)),
        truth_photon_anchor_available: truth.photon.is_some(),
        gen_photon_from_higgs: truth.photon.is_some(),
        nearest_gen_pi_plus_available: truth.pi_plus.is_some(),
        nearest_gen_pi_minus_available: truth.pi_minus.is_some(),
        gen_h: truth.h,
        gen_rho: truth.rho,
        gen_photon: truth.photon,
        gen_pi_plus: truth.pi_plus,
        gen_pi_minus: truth.pi_minus,
        gen_h_proxy: truth.h,
        gen_rho_proxy: truth.rho,
        delta_r_reco_photon_gen_photon: dr_photon,
        delta_r_reco_pi_plus_gen_pi_plus: dr_pi_plus,
        delta_r_reco_pi_minus_gen_pi_minus: dr_pi_minus,
        delta_r_reco_rho_gen_rho: dr_rho,
        delta_r_reco_rho_gen_rho_proxy: dr_rho,
        delta_r_reco_h_gen_h_proxy: truth.h.map(|gen| delta_r(h.eta, h.phi, gen.eta, gen.phi)),
        reco_h_mass_minus_gen_h_mass: truth.h.map(|gen| h.mass - gen.mass),
        reco_rho_mass_minus_gen_rho_mass: truth.rho.map(|gen| h.rho.mass - gen.mass),
        reco_h_mass_minus_gen_h_proxy_mass: truth.h.map(|gen| h.mass - gen.mass),
        reco_rho_mass_minus_gen_rho_proxy_mass: truth.rho.map(|gen| h.rho.mass - gen.mass),
        reco_photon_pt_over_gen_photon_pt: truth
            .photon
            .and_then(|gen| safe_ratio(h.gamma.pt, gen.pt)),
        reco_rho_pt_over_gen_rho_pt: truth.rho.and_then(|gen| safe_ratio(h.rho.pt, gen.pt)),
        reco_pi_plus_pt_over_gen_pi_plus_pt: truth
            .pi_plus
            .and_then(|gen| safe_ratio(h.rho.pi_plus.pt, gen.pt)),
        reco_pi_minus_pt_over_gen_pi_minus_pt: truth
            .pi_minus
            .and_then(|gen| safe_ratio(h.rho.pi_minus.pt, gen.pt)),
        reco_rho_pt_over_gen_rho_proxy_pt: truth.rho.and_then(|gen| safe_ratio(h.rho.pt, gen.pt)),
        reco_h_pt_over_gen_h_proxy_pt: truth.h.and_then(|gen| safe_ratio(h.pt, gen.pt)),
        ..TruthMatchResult::not_available()
    }
}

pub fn match_reco_to_truth_proxy(h: &HCand, particles: Option<&[GenParticle]>) -> TruthMatchResult {
    let Some(particles) = particles else {
        return TruthMatchResult::not_available();
    };

    let photon =
        nearest_gen_particle_by_delta_r(h.gamma.eta, h.gamma.phi, particles, |index, particle| {
            particle.pdg_id == 22
                && is_usable_gen_particle(*particle)
                && has_gen_ancestor(particles, index, 25)
        });
    let pi_plus = nearest_gen_particle_by_delta_r(
        h.rho.pi_plus.eta,
        h.rho.pi_plus.phi,
        particles,
        |_index, particle| particle.pdg_id == 211 && is_usable_final_state_candidate(*particle),
    );
    let pi_minus = nearest_gen_particle_by_delta_r(
        h.rho.pi_minus.eta,
        h.rho.pi_minus.phi,
        particles,
        |_index, particle| particle.pdg_id == -211 && is_usable_final_state_candidate(*particle),
    );

    let gen_photon = photon.map(|matched| matched.particle);
    let gen_pi_plus = pi_plus.map(|matched| matched.particle);
    let gen_pi_minus = pi_minus.map(|matched| matched.particle);
    let gen_rho_proxy = gen_pi_plus
        .zip(gen_pi_minus)
        .map(|(plus, minus)| proxy_particle(113, plus.p4().add(&minus.p4())));
    let gen_h_proxy =
        gen_photon
            .zip(gen_pi_plus)
            .zip(gen_pi_minus)
            .map(|((photon, plus), minus)| {
                proxy_particle(25, photon.p4().add(&plus.p4()).add(&minus.p4()))
            });

    let dr_photon = photon.map(|matched| matched.delta_r);
    let dr_pi_plus = pi_plus.map(|matched| matched.delta_r);
    let dr_pi_minus = pi_minus.map(|matched| matched.delta_r);
    let matched_0p1 = proxy_threshold_match(dr_photon, dr_pi_plus, dr_pi_minus, 0.1);
    let matched_0p2 = proxy_threshold_match(dr_photon, dr_pi_plus, dr_pi_minus, 0.2);
    let matched_0p3 = proxy_threshold_match(dr_photon, dr_pi_plus, dr_pi_minus, 0.3);

    TruthMatchResult {
        truth_strategy: TruthStrategy::TopologyProxy,
        truth_available: true,
        truth_topology: TruthTopology::NotFound,
        truth_matched: matched_0p1,
        truth_proxy_matched: matched_0p1,
        truth_proxy_matched_dr_0p1: matched_0p1,
        truth_proxy_matched_dr_0p2: matched_0p2,
        truth_proxy_matched_dr_0p3: matched_0p3,
        truth_photon_anchor_available: gen_photon.is_some(),
        gen_photon_from_higgs: gen_photon.is_some(),
        nearest_gen_pi_plus_available: gen_pi_plus.is_some(),
        nearest_gen_pi_minus_available: gen_pi_minus.is_some(),
        gen_h: gen_h_proxy,
        gen_rho: gen_rho_proxy,
        gen_photon,
        gen_pi_plus,
        gen_pi_minus,
        gen_h_proxy,
        gen_rho_proxy,
        delta_r_reco_photon_gen_photon: dr_photon,
        delta_r_reco_pi_plus_gen_pi_plus: dr_pi_plus,
        delta_r_reco_pi_minus_gen_pi_minus: dr_pi_minus,
        delta_r_reco_rho_gen_rho: gen_rho_proxy
            .map(|gen| delta_r(h.rho.eta, h.rho.phi, gen.eta, gen.phi)),
        delta_r_reco_rho_gen_rho_proxy: gen_rho_proxy
            .map(|gen| delta_r(h.rho.eta, h.rho.phi, gen.eta, gen.phi)),
        delta_r_reco_h_gen_h_proxy: gen_h_proxy.map(|gen| delta_r(h.eta, h.phi, gen.eta, gen.phi)),
        reco_h_mass_minus_gen_h_mass: gen_h_proxy.map(|gen| h.mass - gen.mass),
        reco_rho_mass_minus_gen_rho_mass: gen_rho_proxy.map(|gen| h.rho.mass - gen.mass),
        reco_h_mass_minus_gen_h_proxy_mass: gen_h_proxy.map(|gen| h.mass - gen.mass),
        reco_rho_mass_minus_gen_rho_proxy_mass: gen_rho_proxy.map(|gen| h.rho.mass - gen.mass),
        reco_photon_pt_over_gen_photon_pt: gen_photon
            .and_then(|gen| safe_ratio(h.gamma.pt, gen.pt)),
        reco_rho_pt_over_gen_rho_pt: gen_rho_proxy.and_then(|gen| safe_ratio(h.rho.pt, gen.pt)),
        reco_pi_plus_pt_over_gen_pi_plus_pt: gen_pi_plus
            .and_then(|gen| safe_ratio(h.rho.pi_plus.pt, gen.pt)),
        reco_pi_minus_pt_over_gen_pi_minus_pt: gen_pi_minus
            .and_then(|gen| safe_ratio(h.rho.pi_minus.pt, gen.pt)),
        reco_rho_pt_over_gen_rho_proxy_pt: gen_rho_proxy
            .and_then(|gen| safe_ratio(h.rho.pt, gen.pt)),
        reco_h_pt_over_gen_h_proxy_pt: gen_h_proxy.and_then(|gen| safe_ratio(h.pt, gen.pt)),
        ..TruthMatchResult::not_available()
    }
}

pub fn hgamma_closure_truth_objects(
    reco_photon: Option<&SimpleCand>,
    particles: Option<&[GenParticle]>,
) -> HgammaClosureTruthObjects {
    let Some(particles) = particles else {
        return HgammaClosureTruthObjects::default();
    };
    let Some((h_index, gen_h)) = select_gen_higgs(particles) else {
        return HgammaClosureTruthObjects::default();
    };
    let gen_gamma = select_higgs_photon(particles, h_index, reco_photon);
    let gen_rho_recoil = gen_gamma
        .map(|gamma| gen_h.p4().sub(&gamma.p4()))
        .filter(valid_recoil_p4)
        .map(|p4| proxy_particle(113, p4));
    let delta_r_reco_photon_gen_photon = reco_photon
        .zip(gen_gamma)
        .map(|(reco, gen)| delta_r(reco.eta, reco.phi, gen.eta, gen.phi));

    HgammaClosureTruthObjects {
        gen_h: Some(gen_h),
        gen_gamma,
        gen_rho_recoil,
        delta_r_reco_photon_gen_photon,
    }
}

pub fn match_reco_to_hgamma_closure(
    h: &HCand,
    particles: Option<&[GenParticle]>,
) -> TruthMatchResult {
    let objects = hgamma_closure_truth_objects(Some(&h.gamma), particles);
    let Some(gen_h) = objects.gen_h else {
        return TruthMatchResult::not_available();
    };
    let Some(gen_gamma) = objects.gen_gamma else {
        return TruthMatchResult {
            truth_strategy: TruthStrategy::HgammaClosure,
            truth_available: false,
            hgamma_gen_h_available: true,
            hgamma_gen_h: Some(gen_h),
            gen_h: Some(gen_h),
            ..TruthMatchResult::not_available()
        };
    };
    let Some(gen_rho_recoil) = objects.gen_rho_recoil else {
        return TruthMatchResult {
            truth_strategy: TruthStrategy::HgammaClosure,
            truth_available: false,
            hgamma_gen_h_available: true,
            hgamma_gen_gamma_available: true,
            hgamma_gen_h: Some(gen_h),
            hgamma_gen_gamma: Some(gen_gamma),
            gen_h: Some(gen_h),
            gen_photon: Some(gen_gamma),
            delta_r_reco_photon_gen_photon: objects.delta_r_reco_photon_gen_photon,
            ..TruthMatchResult::not_available()
        };
    };

    let dr_photon = objects.delta_r_reco_photon_gen_photon;
    let dr_h = delta_r(h.eta, h.phi, gen_h.eta, gen_h.phi);
    let dr_rho = delta_r(h.rho.eta, h.rho.phi, gen_rho_recoil.eta, gen_rho_recoil.phi);
    let h_mass_diff = h.mass - gen_h.mass;
    let rho_mass_diff = h.rho.mass - gen_rho_recoil.mass;
    let photon_matched_0p1 = dr_photon.is_some_and(|dr| dr < 0.1);
    let photon_matched_0p2 = dr_photon.is_some_and(|dr| dr < 0.2);
    let higgs_closed_mass_10 = h_mass_diff.abs() < 10.0;
    let higgs_closed_mass_15 = h_mass_diff.abs() < 15.0;
    let higgs_closed_mass_20 = h_mass_diff.abs() < 20.0;
    let higgs_closed_dr_0p3 = dr_h < 0.3;
    let higgs_closed_dr_0p5 = dr_h < 0.5;
    let hgamma_closure_matched = photon_matched_0p1 && higgs_closed_mass_15;

    TruthMatchResult {
        truth_strategy: TruthStrategy::HgammaClosure,
        truth_available: true,
        truth_topology: TruthTopology::NotFound,
        truth_matched: hgamma_closure_matched,
        truth_photon_anchor_available: true,
        gen_photon_from_higgs: true,
        gen_h: Some(gen_h),
        gen_rho: Some(gen_rho_recoil),
        gen_photon: Some(gen_gamma),
        gen_h_proxy: Some(gen_h),
        gen_rho_proxy: Some(gen_rho_recoil),
        delta_r_reco_photon_gen_photon: dr_photon,
        delta_r_reco_rho_gen_rho: Some(dr_rho),
        delta_r_reco_rho_gen_rho_proxy: Some(dr_rho),
        delta_r_reco_h_gen_h_proxy: Some(dr_h),
        reco_h_mass_minus_gen_h_mass: Some(h_mass_diff),
        reco_rho_mass_minus_gen_rho_mass: Some(rho_mass_diff),
        reco_h_mass_minus_gen_h_proxy_mass: Some(h_mass_diff),
        reco_rho_mass_minus_gen_rho_proxy_mass: Some(rho_mass_diff),
        reco_photon_pt_over_gen_photon_pt: safe_ratio(h.gamma.pt, gen_gamma.pt),
        reco_rho_pt_over_gen_rho_pt: safe_ratio(h.rho.pt, gen_rho_recoil.pt),
        reco_rho_pt_over_gen_rho_proxy_pt: safe_ratio(h.rho.pt, gen_rho_recoil.pt),
        reco_h_pt_over_gen_h_proxy_pt: safe_ratio(h.pt, gen_h.pt),
        hgamma_closure_available: true,
        hgamma_closure_matched,
        hgamma_gen_h_available: true,
        hgamma_gen_gamma_available: true,
        hgamma_gen_rho_recoil_available: true,
        hgamma_photon_matched_dr_0p1: photon_matched_0p1,
        hgamma_photon_matched_dr_0p2: photon_matched_0p2,
        hgamma_higgs_closed_mass_10: higgs_closed_mass_10,
        hgamma_higgs_closed_mass_15: higgs_closed_mass_15,
        hgamma_higgs_closed_mass_20: higgs_closed_mass_20,
        hgamma_higgs_closed_dr_0p3: higgs_closed_dr_0p3,
        hgamma_higgs_closed_dr_0p5: higgs_closed_dr_0p5,
        hgamma_gen_h: Some(gen_h),
        hgamma_gen_gamma: Some(gen_gamma),
        hgamma_gen_rho_recoil: Some(gen_rho_recoil),
        reco_photon_eta_minus_gen_photon_eta: Some(h.gamma.eta - gen_gamma.eta),
        reco_photon_phi_minus_gen_photon_phi: Some(delta_phi(h.gamma.phi, gen_gamma.phi)),
        delta_r_reco_h_gen_h: Some(dr_h),
        reco_h_pt_over_gen_h_pt: safe_ratio(h.pt, gen_h.pt),
        delta_r_reco_rho_gen_rho_recoil: Some(dr_rho),
        reco_rho_mass_minus_gen_rho_recoil_mass: Some(rho_mass_diff),
        reco_rho_pt_over_gen_rho_recoil_pt: safe_ratio(h.rho.pt, gen_rho_recoil.pt),
        ..TruthMatchResult::not_available()
    }
}

pub fn has_gen_ancestor(particles: &[GenParticle], index: usize, ancestor_pdg_id: i32) -> bool {
    let Some(mut current) = particles
        .get(index)
        .and_then(|particle| particle.mother_index)
    else {
        return false;
    };
    let mut seen = 0usize;
    while seen <= particles.len() {
        let Some(particle) = particles.get(current) else {
            return false;
        };
        if particle.pdg_id == ancestor_pdg_id {
            return true;
        }
        let Some(next) = particle.mother_index else {
            return false;
        };
        current = next;
        seen += 1;
    }
    false
}

pub fn safe_ratio(numerator: f64, denominator: f64) -> Option<f64> {
    (denominator.is_finite() && denominator > 0.0).then_some(numerator / denominator)
}

#[derive(Debug, Clone, Copy)]
struct MatchedGenParticle {
    particle: GenParticle,
    delta_r: f64,
}

fn nearest_gen_particle_by_delta_r(
    reco_eta: f64,
    reco_phi: f64,
    particles: &[GenParticle],
    predicate: impl Fn(usize, &GenParticle) -> bool,
) -> Option<MatchedGenParticle> {
    particles
        .iter()
        .enumerate()
        .filter(|(index, particle)| predicate(*index, particle))
        .map(|(_, particle)| MatchedGenParticle {
            particle: *particle,
            delta_r: delta_r(reco_eta, reco_phi, particle.eta, particle.phi),
        })
        .min_by(|a, b| a.delta_r.partial_cmp(&b.delta_r).unwrap_or(Ordering::Equal))
}

fn select_gen_higgs(particles: &[GenParticle]) -> Option<(usize, GenParticle)> {
    particles
        .iter()
        .enumerate()
        .filter(|(_, particle)| particle.pdg_id == 25 && is_usable_gen_particle(**particle))
        .max_by(|(_, a), (_, b)| {
            a.status_flags
                .cmp(&b.status_flags)
                .then_with(|| a.status.cmp(&b.status))
                .then_with(|| a.pt.partial_cmp(&b.pt).unwrap_or(Ordering::Equal))
        })
        .map(|(index, particle)| (index, *particle))
}

fn select_higgs_photon(
    particles: &[GenParticle],
    h_index: usize,
    reco_photon: Option<&SimpleCand>,
) -> Option<GenParticle> {
    if let Some(reco) = reco_photon {
        return nearest_gen_particle_by_delta_r(
            reco.eta,
            reco.phi,
            particles,
            |index, particle| {
                particle.pdg_id == 22
                    && is_usable_gen_particle(*particle)
                    && is_descendant_of(particles, *particle, h_index)
                    && index != h_index
            },
        )
        .map(|matched| matched.particle);
    }

    particles
        .iter()
        .enumerate()
        .filter(|(index, particle)| {
            particle.pdg_id == 22
                && is_usable_gen_particle(**particle)
                && is_descendant_of(particles, **particle, h_index)
                && *index != h_index
        })
        .max_by(|(_, a), (_, b)| a.pt.partial_cmp(&b.pt).unwrap_or(Ordering::Equal))
        .map(|(_, particle)| *particle)
}

fn valid_recoil_p4(p4: &FourVec) -> bool {
    p4.e.is_finite()
        && p4.e > 0.0
        && p4.px.is_finite()
        && p4.py.is_finite()
        && p4.pz.is_finite()
        && p4.pt().is_finite()
        && p4.eta().is_finite()
        && p4.phi().is_finite()
        && p4.mass().is_finite()
}

fn is_usable_gen_particle(particle: GenParticle) -> bool {
    particle.pt > 0.1 && particle.eta.is_finite() && particle.phi.is_finite()
}

fn is_usable_final_state_candidate(particle: GenParticle) -> bool {
    is_usable_gen_particle(particle) && (particle.status == 1 || particle.status == 0)
}

fn proxy_threshold_match(
    dr_photon: Option<f64>,
    dr_pi_plus: Option<f64>,
    dr_pi_minus: Option<f64>,
    threshold: f64,
) -> bool {
    [dr_photon, dr_pi_plus, dr_pi_minus]
        .into_iter()
        .all(|value| value.is_some_and(|dr| dr < threshold))
}

fn proxy_particle(pdg_id: i32, p4: FourVec) -> GenParticle {
    GenParticle::with_status(pdg_id, None, p4.pt(), p4.eta(), p4.phi(), p4.mass(), 0, 0)
}

impl GenParticle {
    fn p4(self) -> FourVec {
        FourVec::from_pt_eta_phi_mass(self.pt, self.eta, self.phi, self.mass)
    }
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

    fn default_cuts_toml() -> &'static str {
        r#"
[baseline.zcountinghlt_naive]
photon_min_pt = 15.0
pi1_min_pt = 5.0
pi2_min_pt = 2.0
max_delta_r_pipi = 0.1
rho_mass_min = 0.3
rho_mass_max = 1.2
min_delta_r_gamma_rho = 1.0
max_delta_r_gamma_rho = 5.0
pion_mass = 0.13957039
rho_mass_target = 0.77526
higgs_mass_reference = 125.0
"#
    }

    #[test]
    fn loads_branch_mapping_from_yaml() {
        let config_toml = r#"
[analysis]
branch_catalogue = "configs/branches/scouting_run3.yaml"

[objects.photon]
source = "ScoutingPhoton"

[objects.charged_candidate]
source = "ScoutingChargedCandidate"

[baseline.zcountinghlt_naive]
photon_min_pt = 15.0
pi1_min_pt = 5.0
pi2_min_pt = 2.0
max_delta_r_pipi = 0.1
rho_mass_min = 0.3
rho_mass_max = 1.2
min_delta_r_gamma_rho = 1.0
max_delta_r_gamma_rho = 5.0
pion_mass = 0.13957039
rho_mass_target = 0.77526
higgs_mass_reference = 125.0
"#;
        // test fallback to workspace root
        let uid = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let config_path = std::env::temp_dir().join(format!("h_rho_gamma_test1_{uid}.toml"));
        std::fs::write(&config_path, config_toml).unwrap();

        let (source, mapping) = load_branch_mapping(&config_path).expect("load branch mapping");
        assert_eq!(source, "configs/branches/scouting_run3.yaml");
        assert_eq!(mapping.photon.semantic_name, "ScoutingPhoton");
        assert_eq!(mapping.photon.count, "nPhoton");
        assert_eq!(
            mapping.charged_candidate.semantic_name,
            "ScoutingChargedCandidate"
        );
        assert_eq!(mapping.charged_candidate.pdg_id, "PFCand_pdgId");
    }

    #[test]
    fn missing_catalogue_object_fails() {
        let config_toml = r#"
[analysis]
branch_catalogue = "configs/branches/scouting_run3.yaml"

[objects.photon]
source = "MissingPhoton"

[objects.charged_candidate]
source = "ScoutingChargedCandidate"

[baseline.zcountinghlt_naive]
photon_min_pt = 15.0
pi1_min_pt = 5.0
pi2_min_pt = 2.0
max_delta_r_pipi = 0.1
rho_mass_min = 0.3
rho_mass_max = 1.2
min_delta_r_gamma_rho = 1.0
max_delta_r_gamma_rho = 5.0
pion_mass = 0.13957039
rho_mass_target = 0.77526
higgs_mass_reference = 125.0
"#;
        let uid = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let config_path = std::env::temp_dir().join(format!("h_rho_gamma_test2_{uid}.toml"));
        std::fs::write(&config_path, config_toml).unwrap();

        let err = load_branch_mapping(&config_path).expect_err("should fail");
        assert!(err
            .to_string()
            .contains("missing photon object: MissingPhoton"));
    }

    #[test]
    fn parses_zcountinghlt_naive_cuts_from_toml() {
        let cuts = HToRhoGammaCuts::from_config_toml_str(default_cuts_toml()).expect("parse cuts");
        assert_eq!(cuts, default_cuts());
    }

    #[test]
    fn committed_config_matches_builtin_zcountinghlt_naive_cuts() {
        let config = include_str!("../../../configs/scouting/h_rho_gamma.toml");
        let cuts = HToRhoGammaCuts::from_config_toml_str(config).expect("parse committed config");
        assert_eq!(cuts, default_cuts());
    }

    #[test]
    fn malformed_config_reports_toml_parse_error() {
        let err = HToRhoGammaCuts::from_config_toml_str("[baseline.zcountinghlt_naive")
            .expect_err("malformed config must fail");
        assert!(err.to_string().contains("failed to parse TOML"));
    }

    #[test]
    fn missing_baseline_field_reports_clear_error() {
        let err = HToRhoGammaCuts::from_config_toml_str(
            r#"
[baseline.zcountinghlt_naive]
photon_min_pt = 15.0
"#,
        )
        .expect_err("missing fields must fail");
        assert!(err.to_string().contains("missing field"));
    }

    #[test]
    fn invalid_cut_values_report_validation_error() {
        let err = HToRhoGammaCuts::from_config_toml_str(
            r#"
[baseline.zcountinghlt_naive]
photon_min_pt = 15.0
pi1_min_pt = 1.0
pi2_min_pt = 2.0
max_delta_r_pipi = 0.1
rho_mass_min = 0.3
rho_mass_max = 1.2
min_delta_r_gamma_rho = 1.0
max_delta_r_gamma_rho = 5.0
pion_mass = 0.13957039
rho_mass_target = 0.77526
higgs_mass_reference = 125.0
"#,
        )
        .expect_err("invalid values must fail");
        assert!(err.to_string().contains("pi1_min_pt must be >= pi2_min_pt"));
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

    #[test]
    fn finds_explicit_h_to_rho_gamma_truth_chain() {
        let truth = identify_truth_chain(&[
            GenParticle::new(25, None, 100.0, 0.0, 0.0, 125.0),
            GenParticle::new(22, Some(0), 60.0, 0.1, 0.1, 0.0),
            GenParticle::new(113, Some(0), 40.0, -0.1, -0.1, 0.775),
            GenParticle::new(211, Some(2), 25.0, -0.08, -0.08, 0.139),
            GenParticle::new(-211, Some(2), 15.0, -0.12, -0.12, 0.139),
        ]);

        assert_eq!(truth.topology, TruthTopology::ExplicitRho);
        assert!(truth.rho.is_some());
        assert!(truth.pi_plus.is_some());
        assert!(truth.pi_minus.is_some());
    }

    #[test]
    fn finds_fallback_truth_chain_without_explicit_rho() {
        let truth = identify_truth_chain(&[
            GenParticle::new(25, None, 100.0, 0.0, 0.0, 125.0),
            GenParticle::new(22, Some(0), 60.0, 0.1, 0.1, 0.0),
            GenParticle::new(211, Some(0), 25.0, -0.08, -0.08, 0.139),
            GenParticle::new(-211, Some(0), 15.0, -0.12, -0.12, 0.139),
        ]);

        assert_eq!(truth.topology, TruthTopology::FallbackNoExplicitRho);
        assert!(truth.rho.is_none());
    }

    #[test]
    fn missing_truth_chain_returns_not_found() {
        let truth = identify_truth_chain(&[
            GenParticle::new(25, None, 100.0, 0.0, 0.0, 125.0),
            GenParticle::new(22, Some(0), 60.0, 0.1, 0.1, 0.0),
        ]);

        assert_eq!(truth.topology, TruthTopology::NotFound);
    }

    #[test]
    fn truth_matching_uses_default_delta_r_threshold() {
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
        let h = reco.h.expect("reco candidate");
        let truth = HToRhoGammaTruth {
            topology: TruthTopology::ExplicitRho,
            h: Some(GenParticle::new(25, None, h.pt, h.eta, h.phi, 125.0)),
            rho: Some(GenParticle::new(
                113,
                Some(0),
                h.rho.pt,
                h.rho.eta,
                h.rho.phi,
                0.775,
            )),
            photon: Some(GenParticle::new(
                22,
                Some(0),
                h.gamma.pt,
                h.gamma.eta,
                h.gamma.phi,
                0.0,
            )),
            pi_plus: Some(GenParticle::new(
                211,
                Some(1),
                h.rho.pi_plus.pt,
                h.rho.pi_plus.eta,
                h.rho.pi_plus.phi,
                0.139,
            )),
            pi_minus: Some(GenParticle::new(
                -211,
                Some(1),
                h.rho.pi_minus.pt,
                h.rho.pi_minus.eta,
                h.rho.pi_minus.phi,
                0.139,
            )),
        };

        let matched = match_reco_to_truth(&h, Some(&truth));
        assert!(matched.truth_matched);
        assert_eq!(matched.truth_topology, TruthTopology::ExplicitRho);

        let mut shifted_truth = truth;
        shifted_truth.photon = Some(GenParticle::new(22, Some(0), h.gamma.pt, 1.0, 1.0, 0.0));
        let mismatched = match_reco_to_truth(&h, Some(&shifted_truth));
        assert!(!mismatched.truth_matched);
    }

    #[test]
    fn truth_proxy_matches_higgs_photon_and_nearest_pions_without_pion_ancestry() {
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
        let h = reco.h.expect("reco candidate");
        let particles = [
            GenParticle::with_status(25, None, 125.0, 0.0, 0.0, 125.0, 2, 26881),
            GenParticle::with_status(22, Some(0), h.gamma.pt, 0.02, 0.02, 0.0, 1, 12289),
            GenParticle::with_status(
                211,
                None,
                h.rho.pi_plus.pt,
                h.rho.pi_plus.eta + 0.02,
                h.rho.pi_plus.phi + 0.02,
                0.13957039,
                1,
                12308,
            ),
            GenParticle::with_status(
                -211,
                None,
                h.rho.pi_minus.pt,
                h.rho.pi_minus.eta - 0.02,
                h.rho.pi_minus.phi - 0.02,
                0.13957039,
                1,
                12308,
            ),
        ];

        let proxy = match_reco_to_truth_proxy(&h, Some(&particles));

        assert_eq!(proxy.truth_strategy, TruthStrategy::TopologyProxy);
        assert!(proxy.truth_available);
        assert!(proxy.truth_photon_anchor_available);
        assert!(proxy.gen_photon_from_higgs);
        assert!(proxy.nearest_gen_pi_plus_available);
        assert!(proxy.nearest_gen_pi_minus_available);
        assert!(proxy.truth_proxy_matched);
        assert!(proxy.truth_proxy_matched_dr_0p1);
        assert!(proxy.truth_proxy_matched_dr_0p2);
        assert!(proxy.truth_proxy_matched_dr_0p3);
        assert!(proxy.gen_rho_proxy.is_some());
        assert!(proxy.gen_h_proxy.is_some());
        assert!(proxy.delta_r_reco_rho_gen_rho_proxy.is_some());
        assert!(proxy.delta_r_reco_h_gen_h_proxy.is_some());
        assert!(proxy.reco_pi_plus_pt_over_gen_pi_plus_pt.is_some());
        assert!(proxy.reco_pi_minus_pt_over_gen_pi_minus_pt.is_some());
    }

    #[test]
    fn truth_proxy_threshold_flags_are_reported_independently() {
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
        let h = reco.h.expect("reco candidate");
        let particles = [
            GenParticle::with_status(25, None, 125.0, 0.0, 0.0, 125.0, 2, 26881),
            GenParticle::with_status(22, Some(0), h.gamma.pt, 0.15, 0.0, 0.0, 1, 12289),
            GenParticle::with_status(
                211,
                None,
                h.rho.pi_plus.pt,
                h.rho.pi_plus.eta,
                h.rho.pi_plus.phi + 0.15,
                0.13957039,
                1,
                12308,
            ),
            GenParticle::with_status(
                -211,
                None,
                h.rho.pi_minus.pt,
                h.rho.pi_minus.eta,
                h.rho.pi_minus.phi + 0.15,
                0.13957039,
                1,
                12308,
            ),
        ];

        let proxy = match_reco_to_truth_proxy(&h, Some(&particles));

        assert!(!proxy.truth_proxy_matched);
        assert!(!proxy.truth_proxy_matched_dr_0p1);
        assert!(proxy.truth_proxy_matched_dr_0p2);
        assert!(proxy.truth_proxy_matched_dr_0p3);
    }

    #[test]
    fn truth_proxy_handles_missing_gen_and_safe_ratio() {
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
        let h = reco.h.expect("reco candidate");

        let proxy = match_reco_to_truth_proxy(&h, None);

        assert_eq!(proxy.truth_strategy, TruthStrategy::None);
        assert!(!proxy.truth_available);
        assert!(!proxy.truth_proxy_matched);
        assert_eq!(safe_ratio(1.0, 0.0), None);
        assert_eq!(safe_ratio(2.0, 4.0), Some(0.5));
    }

    #[test]
    fn hgamma_closure_builds_recoil_and_closure_flags() {
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
        let h = reco.h.expect("reco candidate");
        let particles = [
            GenParticle::with_status(25, None, h.pt, h.eta, h.phi, h.mass, 2, 26881),
            GenParticle::with_status(
                22,
                Some(0),
                h.gamma.pt,
                h.gamma.eta + 0.02,
                h.gamma.phi + 0.02,
                0.0,
                1,
                12289,
            ),
        ];

        let closure = match_reco_to_hgamma_closure(&h, Some(&particles));

        assert_eq!(closure.truth_strategy, TruthStrategy::HgammaClosure);
        assert!(closure.hgamma_closure_available);
        assert!(closure.hgamma_closure_matched);
        assert!(closure.hgamma_gen_h_available);
        assert!(closure.hgamma_gen_gamma_available);
        assert!(closure.hgamma_gen_rho_recoil_available);
        assert!(closure.hgamma_photon_matched_dr_0p1);
        assert!(closure.hgamma_photon_matched_dr_0p2);
        assert!(closure.hgamma_higgs_closed_mass_10);
        assert!(closure.hgamma_higgs_closed_mass_15);
        assert!(closure.hgamma_higgs_closed_mass_20);
        assert!(closure.hgamma_higgs_closed_dr_0p3);
        assert!(closure.hgamma_higgs_closed_dr_0p5);
        assert!(closure.hgamma_gen_rho_recoil.is_some());
        assert!(closure.delta_r_reco_rho_gen_rho_recoil.is_some());
        assert!(closure.reco_rho_pt_over_gen_rho_recoil_pt.is_some());
    }

    #[test]
    fn hgamma_closure_handles_missing_gen() {
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
        let h = reco.h.expect("reco candidate");

        let closure = match_reco_to_hgamma_closure(&h, None);

        assert_eq!(closure.truth_strategy, TruthStrategy::None);
        assert!(!closure.hgamma_closure_available);
        assert!(!closure.hgamma_closure_matched);
        assert!(!closure.hgamma_gen_h_available);
        assert!(!closure.hgamma_gen_gamma_available);
        assert!(!closure.hgamma_gen_rho_recoil_available);
    }

    #[test]
    fn hgamma_event_counters_update_from_event_state() {
        let mut counters = HgammaClosureCounters::default();
        counters.observe(HgammaClosureEventFlags {
            has_gen_h: true,
            has_gen_hgamma: true,
            has_reco_photon_preselection: true,
            photon_matched_dr_0p1: true,
            photon_matched_dr_0p2: true,
            has_os_track_pair: true,
            has_accepted_candidate: true,
            higgs_closed_mass_10: false,
            higgs_closed_mass_15: true,
            higgs_closed_mass_20: true,
        });
        counters.observe(HgammaClosureEventFlags {
            has_gen_h: true,
            has_gen_hgamma: false,
            has_reco_photon_preselection: false,
            photon_matched_dr_0p1: false,
            photon_matched_dr_0p2: false,
            has_os_track_pair: false,
            has_accepted_candidate: false,
            higgs_closed_mass_10: false,
            higgs_closed_mass_15: false,
            higgs_closed_mass_20: false,
        });

        assert_eq!(counters.events_total, 2);
        assert_eq!(counters.events_with_gen_h, 2);
        assert_eq!(counters.events_with_gen_hgamma, 1);
        assert_eq!(counters.events_with_reco_photon_preselection, 1);
        assert_eq!(counters.events_with_reco_photon_matched_dr_0p1, 1);
        assert_eq!(counters.events_with_reco_photon_matched_dr_0p2, 1);
        assert_eq!(
            counters.events_with_reco_photon_matched_dr_0p1_and_any_os_track_pair,
            1
        );
        assert_eq!(
            counters.events_with_reco_photon_matched_dr_0p1_and_accepted_candidate,
            1
        );
        assert_eq!(
            counters.events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_10,
            0
        );
        assert_eq!(
            counters.events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_15,
            1
        );
        assert_eq!(
            counters.events_with_reco_photon_matched_dr_0p1_and_higgs_closed_mass_20,
            1
        );
    }
}
