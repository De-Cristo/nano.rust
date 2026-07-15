use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use nano_io::writer::{write_events, OutputBranch};

const HEADER: [&str; 23] = [
    "run",
    "luminosityBlock",
    "event",
    "photon_pt",
    "photon_eta",
    "photon_phi",
    "pi_plus_pt",
    "pi_plus_eta",
    "pi_plus_phi",
    "pi_minus_pt",
    "pi_minus_eta",
    "pi_minus_phi",
    "rho_mass",
    "rho_pt",
    "rho_eta",
    "rho_phi",
    "h_mass",
    "h_pt",
    "h_eta",
    "h_phi",
    "delta_r_pipi",
    "delta_r_gamma_rho",
    "rho_pt_over_photon_pt",
];

const STAGE14_TRUTH_HEADER: [&str; 28] = [
    "truth_available",
    "truth_topology",
    "truth_matched",
    "gen_h_pt",
    "gen_h_eta",
    "gen_h_phi",
    "gen_h_mass",
    "gen_rho_pt",
    "gen_rho_eta",
    "gen_rho_phi",
    "gen_rho_mass",
    "gen_photon_pt",
    "gen_photon_eta",
    "gen_photon_phi",
    "gen_pi_plus_pt",
    "gen_pi_plus_eta",
    "gen_pi_plus_phi",
    "gen_pi_minus_pt",
    "gen_pi_minus_eta",
    "gen_pi_minus_phi",
    "delta_r_reco_photon_gen_photon",
    "delta_r_reco_pi_plus_gen_pi_plus",
    "delta_r_reco_pi_minus_gen_pi_minus",
    "delta_r_reco_rho_gen_rho",
    "reco_h_mass_minus_gen_h_mass",
    "reco_rho_mass_minus_gen_rho_mass",
    "reco_photon_pt_over_gen_photon_pt",
    "reco_rho_pt_over_gen_rho_pt",
];

const STAGE15_PROXY_HEADER: [&str; 42] = [
    "truth_strategy",
    "truth_available",
    "truth_proxy_matched",
    "truth_proxy_matched_dr_0p1",
    "truth_proxy_matched_dr_0p2",
    "truth_proxy_matched_dr_0p3",
    "truth_photon_anchor_available",
    "gen_photon_from_higgs",
    "gen_photon_pt",
    "gen_photon_eta",
    "gen_photon_phi",
    "gen_photon_mass",
    "nearest_gen_pi_plus_available",
    "gen_pi_plus_pt",
    "gen_pi_plus_eta",
    "gen_pi_plus_phi",
    "gen_pi_plus_mass",
    "nearest_gen_pi_minus_available",
    "gen_pi_minus_pt",
    "gen_pi_minus_eta",
    "gen_pi_minus_phi",
    "gen_pi_minus_mass",
    "gen_rho_proxy_pt",
    "gen_rho_proxy_eta",
    "gen_rho_proxy_phi",
    "gen_rho_proxy_mass",
    "gen_h_proxy_pt",
    "gen_h_proxy_eta",
    "gen_h_proxy_phi",
    "gen_h_proxy_mass",
    "delta_r_reco_photon_gen_photon",
    "delta_r_reco_pi_plus_gen_pi_plus",
    "delta_r_reco_pi_minus_gen_pi_minus",
    "delta_r_reco_rho_gen_rho_proxy",
    "delta_r_reco_h_gen_h_proxy",
    "reco_h_mass_minus_gen_h_proxy_mass",
    "reco_rho_mass_minus_gen_rho_proxy_mass",
    "reco_photon_pt_over_gen_photon_pt",
    "reco_pi_plus_pt_over_gen_pi_plus_pt",
    "reco_pi_minus_pt_over_gen_pi_minus_pt",
    "reco_rho_pt_over_gen_rho_proxy_pt",
    "reco_h_pt_over_gen_h_proxy_pt",
];

const STAGE16A_HGAMMA_HEADER: [&str; 35] = [
    "truth_strategy",
    "hgamma_closure_available",
    "hgamma_closure_matched",
    "hgamma_gen_h_available",
    "hgamma_gen_gamma_available",
    "hgamma_gen_rho_recoil_available",
    "hgamma_photon_matched_dr_0p1",
    "hgamma_photon_matched_dr_0p2",
    "hgamma_higgs_closed_mass_10",
    "hgamma_higgs_closed_mass_15",
    "hgamma_higgs_closed_mass_20",
    "hgamma_higgs_closed_dr_0p3",
    "hgamma_higgs_closed_dr_0p5",
    "hgamma_gen_h_pt",
    "hgamma_gen_h_eta",
    "hgamma_gen_h_phi",
    "hgamma_gen_h_mass",
    "hgamma_gen_gamma_pt",
    "hgamma_gen_gamma_eta",
    "hgamma_gen_gamma_phi",
    "hgamma_gen_gamma_mass",
    "hgamma_gen_rho_recoil_pt",
    "hgamma_gen_rho_recoil_eta",
    "hgamma_gen_rho_recoil_phi",
    "hgamma_gen_rho_recoil_mass",
    "delta_r_reco_photon_gen_photon",
    "reco_photon_pt_over_gen_photon_pt",
    "reco_photon_eta_minus_gen_photon_eta",
    "reco_photon_phi_minus_gen_photon_phi",
    "delta_r_reco_h_gen_h",
    "reco_h_mass_minus_gen_h_mass",
    "reco_h_pt_over_gen_h_pt",
    "delta_r_reco_rho_gen_rho_recoil",
    "reco_rho_mass_minus_gen_rho_recoil_mass",
    "reco_rho_pt_over_gen_rho_recoil_pt",
];

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!(
            "usage: h_rho_gamma_csv_to_root <combined_candidates.csv> <combined_candidates.root>"
        );
        return Ok(());
    }
    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    let table = CandidateTable::read_csv(&input)?;
    let entries = table.entries;
    table.write_root(&output)?;
    println!("wrote {entries} entries to {}", output.display());
    Ok(())
}

#[derive(Debug, Default)]
struct CandidateTable {
    entries: usize,
    columns: BTreeMap<String, Vec<String>>,
    header: Vec<String>,
}

impl CandidateTable {
    fn read_csv(path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let mut lines = content.lines();
        let header = lines
            .next()
            .ok_or_else(|| format!("candidate CSV is empty: {}", path.display()))?
            .split(',')
            .map(str::to_string)
            .collect::<Vec<_>>();
        validate_header(&header)?;

        let mut table = Self {
            header: header.clone(),
            ..Self::default()
        };
        for column in &header {
            table.columns.insert(column.clone(), Vec::new());
        }
        for (line_index, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let row_number = line_index + 2;
            let values = line.split(',').collect::<Vec<_>>();
            if values.len() != header.len() {
                return Err(format!(
                    "row {row_number}: expected {} columns, found {}",
                    header.len(),
                    values.len()
                )
                .into());
            }
            for (column, value) in header.iter().zip(values) {
                table
                    .columns
                    .get_mut(column)
                    .expect("header column initialized")
                    .push(value.to_string());
            }
            table.entries += 1;
        }
        Ok(table)
    }

    fn write_root(self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let mut branches = Vec::new();
        branches.push(OutputBranch::u32("run", parse_u32(self.values("run"))?));
        branches.push(OutputBranch::u32(
            "luminosityBlock",
            parse_u32(self.values("luminosityBlock"))?,
        ));
        branches.push(OutputBranch::u64("event", parse_u64(self.values("event"))?));

        for column in &self.header {
            if matches!(column.as_str(), "run" | "luminosityBlock" | "event") {
                continue;
            }
            if column == "truth_topology" {
                branches.push(OutputBranch::i32(
                    "truth_topology_code",
                    self.values(column)
                        .iter()
                        .map(|value| topology_code(value))
                        .collect(),
                ));
                continue;
            }
            if column == "truth_strategy" {
                branches.push(OutputBranch::i32(
                    "truth_strategy_code",
                    self.values(column)
                        .iter()
                        .map(|value| strategy_code(value))
                        .collect(),
                ));
                continue;
            }
            if is_bool_column(column) {
                branches.push(OutputBranch::bool(column, parse_bool(self.values(column))?));
            } else {
                branches.push(OutputBranch::f32(column, parse_f32(self.values(column))?));
            }
        }
        write_events(path, &branches)?;
        Ok(())
    }

    fn values(&self, column: &str) -> &[String] {
        self.columns
            .get(column)
            .unwrap_or_else(|| panic!("missing initialized column {column}"))
    }
}

fn validate_header(header: &[String]) -> Result<(), Box<dyn Error>> {
    let base = HEADER.iter().copied().collect::<Vec<_>>();
    let stage14 = HEADER
        .iter()
        .chain(STAGE14_TRUTH_HEADER.iter())
        .copied()
        .collect::<Vec<_>>();
    let stage15 = HEADER
        .iter()
        .chain(STAGE15_PROXY_HEADER.iter())
        .copied()
        .collect::<Vec<_>>();
    let stage16a = HEADER
        .iter()
        .chain(STAGE16A_HGAMMA_HEADER.iter())
        .copied()
        .collect::<Vec<_>>();
    let as_str = header.iter().map(String::as_str).collect::<Vec<_>>();
    if as_str == base || as_str == stage14 || as_str == stage15 || as_str == stage16a {
        Ok(())
    } else {
        Err(format!("unexpected candidate CSV header: {}", header.join(",")).into())
    }
}

fn is_bool_column(column: &str) -> bool {
    matches!(
        column,
        "truth_available"
            | "truth_matched"
            | "truth_proxy_matched"
            | "truth_proxy_matched_dr_0p1"
            | "truth_proxy_matched_dr_0p2"
            | "truth_proxy_matched_dr_0p3"
            | "truth_photon_anchor_available"
            | "gen_photon_from_higgs"
            | "nearest_gen_pi_plus_available"
            | "nearest_gen_pi_minus_available"
            | "hgamma_closure_available"
            | "hgamma_closure_matched"
            | "hgamma_gen_h_available"
            | "hgamma_gen_gamma_available"
            | "hgamma_gen_rho_recoil_available"
            | "hgamma_photon_matched_dr_0p1"
            | "hgamma_photon_matched_dr_0p2"
            | "hgamma_higgs_closed_mass_10"
            | "hgamma_higgs_closed_mass_15"
            | "hgamma_higgs_closed_mass_20"
            | "hgamma_higgs_closed_dr_0p3"
            | "hgamma_higgs_closed_dr_0p5"
    )
}

fn parse_u32(values: &[String]) -> Result<Vec<u32>, Box<dyn Error>> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| "failed to parse u32".into())
        })
        .collect()
}

fn parse_u64(values: &[String]) -> Result<Vec<u64>, Box<dyn Error>> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| "failed to parse u64".into())
        })
        .collect()
}

fn parse_f32(values: &[String]) -> Result<Vec<f32>, Box<dyn Error>> {
    values
        .iter()
        .map(|value| {
            if value.is_empty() {
                Ok(f32::NAN)
            } else {
                value
                    .parse::<f32>()
                    .map_err(|_| format!("failed to parse f32 value {value:?}").into())
            }
        })
        .collect()
}

fn parse_bool(values: &[String]) -> Result<Vec<bool>, Box<dyn Error>> {
    Ok(values
        .iter()
        .map(|value| matches!(value.as_str(), "1" | "true" | "True"))
        .collect())
}

fn topology_code(value: &str) -> i32 {
    match value {
        "explicit_rho" => 1,
        "fallback_no_explicit_rho" => 2,
        "not_found" => 3,
        _ => 0,
    }
}

fn strategy_code(value: &str) -> i32 {
    match value {
        "topology_proxy" => 1,
        "hgamma_closure" => 2,
        "explicit_chain" => 3,
        _ => 0,
    }
}
