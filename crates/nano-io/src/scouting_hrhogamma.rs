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
}
