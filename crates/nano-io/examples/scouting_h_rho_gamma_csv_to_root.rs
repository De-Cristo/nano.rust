use std::collections::HashMap;
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
const TRUTH_HEADER: [&str; 28] = [
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

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("usage: scouting_h_rho_gamma_csv_to_root <combined_candidates.csv> <combined_candidates.root>");
        return Ok(());
    }
    let input = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    let table = CandidateTable::read_csv(&input)?;
    let entries = table.run.len();
    table.write_root(&output)?;
    println!("wrote {entries} entries to {}", output.display());
    Ok(())
}

#[derive(Debug, Default)]
struct CandidateTable {
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
    truth_enabled: bool,
    truth_available: Vec<bool>,
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
    gen_pi_plus_pt: Vec<f32>,
    gen_pi_plus_eta: Vec<f32>,
    gen_pi_plus_phi: Vec<f32>,
    gen_pi_minus_pt: Vec<f32>,
    gen_pi_minus_eta: Vec<f32>,
    gen_pi_minus_phi: Vec<f32>,
    delta_r_reco_photon_gen_photon: Vec<f32>,
    delta_r_reco_pi_plus_gen_pi_plus: Vec<f32>,
    delta_r_reco_pi_minus_gen_pi_minus: Vec<f32>,
    delta_r_reco_rho_gen_rho: Vec<f32>,
    reco_h_mass_minus_gen_h_mass: Vec<f32>,
    reco_rho_mass_minus_gen_rho_mass: Vec<f32>,
    reco_photon_pt_over_gen_photon_pt: Vec<f32>,
    reco_rho_pt_over_gen_rho_pt: Vec<f32>,
}

impl CandidateTable {
    fn read_csv(path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let mut lines = content.lines();
        let header = lines
            .next()
            .ok_or_else(|| format!("candidate CSV is empty: {}", path.display()))?;
        let truth_enabled = validate_header(header)?;

        let mut table = Self {
            truth_enabled,
            ..Self::default()
        };
        for (line_index, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let row_number = line_index + 2;
            let row = parse_row(line, row_number)?;
            table.push(row);
        }
        Ok(table)
    }

    fn push(&mut self, row: CsvRow) {
        self.run.push(row.run);
        self.luminosity_block.push(row.luminosity_block);
        self.event.push(row.event);
        self.photon_pt.push(row.photon_pt);
        self.photon_eta.push(row.photon_eta);
        self.photon_phi.push(row.photon_phi);
        self.pi_plus_pt.push(row.pi_plus_pt);
        self.pi_plus_eta.push(row.pi_plus_eta);
        self.pi_plus_phi.push(row.pi_plus_phi);
        self.pi_minus_pt.push(row.pi_minus_pt);
        self.pi_minus_eta.push(row.pi_minus_eta);
        self.pi_minus_phi.push(row.pi_minus_phi);
        self.rho_mass.push(row.rho_mass);
        self.rho_pt.push(row.rho_pt);
        self.rho_eta.push(row.rho_eta);
        self.rho_phi.push(row.rho_phi);
        self.h_mass.push(row.h_mass);
        self.h_pt.push(row.h_pt);
        self.h_eta.push(row.h_eta);
        self.h_phi.push(row.h_phi);
        self.delta_r_pipi.push(row.delta_r_pipi);
        self.delta_r_gamma_rho.push(row.delta_r_gamma_rho);
        self.rho_pt_over_photon_pt.push(row.rho_pt_over_photon_pt);
        if let Some(truth) = row.truth {
            self.truth_available.push(truth.truth_available);
            self.truth_matched.push(truth.truth_matched);
            self.truth_topology_code.push(truth.truth_topology_code);
            self.gen_h_pt.push(truth.gen_h_pt);
            self.gen_h_eta.push(truth.gen_h_eta);
            self.gen_h_phi.push(truth.gen_h_phi);
            self.gen_h_mass.push(truth.gen_h_mass);
            self.gen_rho_pt.push(truth.gen_rho_pt);
            self.gen_rho_eta.push(truth.gen_rho_eta);
            self.gen_rho_phi.push(truth.gen_rho_phi);
            self.gen_rho_mass.push(truth.gen_rho_mass);
            self.gen_photon_pt.push(truth.gen_photon_pt);
            self.gen_photon_eta.push(truth.gen_photon_eta);
            self.gen_photon_phi.push(truth.gen_photon_phi);
            self.gen_pi_plus_pt.push(truth.gen_pi_plus_pt);
            self.gen_pi_plus_eta.push(truth.gen_pi_plus_eta);
            self.gen_pi_plus_phi.push(truth.gen_pi_plus_phi);
            self.gen_pi_minus_pt.push(truth.gen_pi_minus_pt);
            self.gen_pi_minus_eta.push(truth.gen_pi_minus_eta);
            self.gen_pi_minus_phi.push(truth.gen_pi_minus_phi);
            self.delta_r_reco_photon_gen_photon
                .push(truth.delta_r_reco_photon_gen_photon);
            self.delta_r_reco_pi_plus_gen_pi_plus
                .push(truth.delta_r_reco_pi_plus_gen_pi_plus);
            self.delta_r_reco_pi_minus_gen_pi_minus
                .push(truth.delta_r_reco_pi_minus_gen_pi_minus);
            self.delta_r_reco_rho_gen_rho
                .push(truth.delta_r_reco_rho_gen_rho);
            self.reco_h_mass_minus_gen_h_mass
                .push(truth.reco_h_mass_minus_gen_h_mass);
            self.reco_rho_mass_minus_gen_rho_mass
                .push(truth.reco_rho_mass_minus_gen_rho_mass);
            self.reco_photon_pt_over_gen_photon_pt
                .push(truth.reco_photon_pt_over_gen_photon_pt);
            self.reco_rho_pt_over_gen_rho_pt
                .push(truth.reco_rho_pt_over_gen_rho_pt);
        }
    }

    fn write_root(mut self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
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
                OutputBranch::f32("gen_pi_plus_pt", std::mem::take(&mut self.gen_pi_plus_pt)),
                OutputBranch::f32("gen_pi_plus_eta", std::mem::take(&mut self.gen_pi_plus_eta)),
                OutputBranch::f32("gen_pi_plus_phi", std::mem::take(&mut self.gen_pi_plus_phi)),
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
                    "reco_h_mass_minus_gen_h_mass",
                    std::mem::take(&mut self.reco_h_mass_minus_gen_h_mass),
                ),
                OutputBranch::f32(
                    "reco_rho_mass_minus_gen_rho_mass",
                    std::mem::take(&mut self.reco_rho_mass_minus_gen_rho_mass),
                ),
                OutputBranch::f32(
                    "reco_photon_pt_over_gen_photon_pt",
                    std::mem::take(&mut self.reco_photon_pt_over_gen_photon_pt),
                ),
                OutputBranch::f32(
                    "reco_rho_pt_over_gen_rho_pt",
                    std::mem::take(&mut self.reco_rho_pt_over_gen_rho_pt),
                ),
            ]);
        }
        write_events(path, &branches)?;
        Ok(())
    }
}

struct CsvRow {
    run: u32,
    luminosity_block: u32,
    event: u64,
    photon_pt: f32,
    photon_eta: f32,
    photon_phi: f32,
    pi_plus_pt: f32,
    pi_plus_eta: f32,
    pi_plus_phi: f32,
    pi_minus_pt: f32,
    pi_minus_eta: f32,
    pi_minus_phi: f32,
    rho_mass: f32,
    rho_pt: f32,
    rho_eta: f32,
    rho_phi: f32,
    h_mass: f32,
    h_pt: f32,
    h_eta: f32,
    h_phi: f32,
    delta_r_pipi: f32,
    delta_r_gamma_rho: f32,
    rho_pt_over_photon_pt: f32,
    truth: Option<TruthRow>,
}

struct TruthRow {
    truth_available: bool,
    truth_matched: bool,
    truth_topology_code: i32,
    gen_h_pt: f32,
    gen_h_eta: f32,
    gen_h_phi: f32,
    gen_h_mass: f32,
    gen_rho_pt: f32,
    gen_rho_eta: f32,
    gen_rho_phi: f32,
    gen_rho_mass: f32,
    gen_photon_pt: f32,
    gen_photon_eta: f32,
    gen_photon_phi: f32,
    gen_pi_plus_pt: f32,
    gen_pi_plus_eta: f32,
    gen_pi_plus_phi: f32,
    gen_pi_minus_pt: f32,
    gen_pi_minus_eta: f32,
    gen_pi_minus_phi: f32,
    delta_r_reco_photon_gen_photon: f32,
    delta_r_reco_pi_plus_gen_pi_plus: f32,
    delta_r_reco_pi_minus_gen_pi_minus: f32,
    delta_r_reco_rho_gen_rho: f32,
    reco_h_mass_minus_gen_h_mass: f32,
    reco_rho_mass_minus_gen_rho_mass: f32,
    reco_photon_pt_over_gen_photon_pt: f32,
    reco_rho_pt_over_gen_rho_pt: f32,
}

fn validate_header(header: &str) -> Result<bool, Box<dyn Error>> {
    let columns = header.split(',').collect::<Vec<_>>();
    if columns == HEADER {
        Ok(false)
    } else if columns
        == HEADER
            .iter()
            .chain(TRUTH_HEADER.iter())
            .copied()
            .collect::<Vec<_>>()
    {
        Ok(true)
    } else {
        Err(format!("unexpected candidate CSV header: {header}").into())
    }
}

fn parse_row(line: &str, row_number: usize) -> Result<CsvRow, Box<dyn Error>> {
    let values = line.split(',').collect::<Vec<_>>();
    let truth_enabled = values.len() == HEADER.len() + TRUTH_HEADER.len();
    if values.len() != HEADER.len() && !truth_enabled {
        return Err(format!(
            "row {row_number}: expected {} or {} columns, found {}",
            HEADER.len(),
            HEADER.len() + TRUTH_HEADER.len(),
            values.len()
        )
        .into());
    }
    let header_names = HEADER
        .iter()
        .chain(
            truth_enabled
                .then_some(TRUTH_HEADER.iter())
                .into_iter()
                .flatten(),
        )
        .copied()
        .collect::<Vec<_>>();
    let map = header_names
        .iter()
        .copied()
        .zip(values.iter().copied())
        .collect::<HashMap<_, _>>();
    Ok(CsvRow {
        run: parse_field(&map, "run", row_number)?,
        luminosity_block: parse_field(&map, "luminosityBlock", row_number)?,
        event: parse_field(&map, "event", row_number)?,
        photon_pt: parse_field(&map, "photon_pt", row_number)?,
        photon_eta: parse_field(&map, "photon_eta", row_number)?,
        photon_phi: parse_field(&map, "photon_phi", row_number)?,
        pi_plus_pt: parse_field(&map, "pi_plus_pt", row_number)?,
        pi_plus_eta: parse_field(&map, "pi_plus_eta", row_number)?,
        pi_plus_phi: parse_field(&map, "pi_plus_phi", row_number)?,
        pi_minus_pt: parse_field(&map, "pi_minus_pt", row_number)?,
        pi_minus_eta: parse_field(&map, "pi_minus_eta", row_number)?,
        pi_minus_phi: parse_field(&map, "pi_minus_phi", row_number)?,
        rho_mass: parse_field(&map, "rho_mass", row_number)?,
        rho_pt: parse_field(&map, "rho_pt", row_number)?,
        rho_eta: parse_field(&map, "rho_eta", row_number)?,
        rho_phi: parse_field(&map, "rho_phi", row_number)?,
        h_mass: parse_field(&map, "h_mass", row_number)?,
        h_pt: parse_field(&map, "h_pt", row_number)?,
        h_eta: parse_field(&map, "h_eta", row_number)?,
        h_phi: parse_field(&map, "h_phi", row_number)?,
        delta_r_pipi: parse_field(&map, "delta_r_pipi", row_number)?,
        delta_r_gamma_rho: parse_field(&map, "delta_r_gamma_rho", row_number)?,
        rho_pt_over_photon_pt: parse_field(&map, "rho_pt_over_photon_pt", row_number)?,
        truth: truth_enabled
            .then(|| parse_truth(&map, row_number))
            .transpose()?,
    })
}

fn parse_truth(row: &HashMap<&str, &str>, row_number: usize) -> Result<TruthRow, Box<dyn Error>> {
    Ok(TruthRow {
        truth_available: parse_bool_field(row, "truth_available", row_number)?,
        truth_matched: parse_bool_field(row, "truth_matched", row_number)?,
        truth_topology_code: topology_code(
            row.get("truth_topology")
                .ok_or_else(|| format!("row {row_number}: missing column truth_topology"))?,
        ),
        gen_h_pt: parse_optional_f32(row, "gen_h_pt", row_number)?,
        gen_h_eta: parse_optional_f32(row, "gen_h_eta", row_number)?,
        gen_h_phi: parse_optional_f32(row, "gen_h_phi", row_number)?,
        gen_h_mass: parse_optional_f32(row, "gen_h_mass", row_number)?,
        gen_rho_pt: parse_optional_f32(row, "gen_rho_pt", row_number)?,
        gen_rho_eta: parse_optional_f32(row, "gen_rho_eta", row_number)?,
        gen_rho_phi: parse_optional_f32(row, "gen_rho_phi", row_number)?,
        gen_rho_mass: parse_optional_f32(row, "gen_rho_mass", row_number)?,
        gen_photon_pt: parse_optional_f32(row, "gen_photon_pt", row_number)?,
        gen_photon_eta: parse_optional_f32(row, "gen_photon_eta", row_number)?,
        gen_photon_phi: parse_optional_f32(row, "gen_photon_phi", row_number)?,
        gen_pi_plus_pt: parse_optional_f32(row, "gen_pi_plus_pt", row_number)?,
        gen_pi_plus_eta: parse_optional_f32(row, "gen_pi_plus_eta", row_number)?,
        gen_pi_plus_phi: parse_optional_f32(row, "gen_pi_plus_phi", row_number)?,
        gen_pi_minus_pt: parse_optional_f32(row, "gen_pi_minus_pt", row_number)?,
        gen_pi_minus_eta: parse_optional_f32(row, "gen_pi_minus_eta", row_number)?,
        gen_pi_minus_phi: parse_optional_f32(row, "gen_pi_minus_phi", row_number)?,
        delta_r_reco_photon_gen_photon: parse_optional_f32(
            row,
            "delta_r_reco_photon_gen_photon",
            row_number,
        )?,
        delta_r_reco_pi_plus_gen_pi_plus: parse_optional_f32(
            row,
            "delta_r_reco_pi_plus_gen_pi_plus",
            row_number,
        )?,
        delta_r_reco_pi_minus_gen_pi_minus: parse_optional_f32(
            row,
            "delta_r_reco_pi_minus_gen_pi_minus",
            row_number,
        )?,
        delta_r_reco_rho_gen_rho: parse_optional_f32(row, "delta_r_reco_rho_gen_rho", row_number)?,
        reco_h_mass_minus_gen_h_mass: parse_optional_f32(
            row,
            "reco_h_mass_minus_gen_h_mass",
            row_number,
        )?,
        reco_rho_mass_minus_gen_rho_mass: parse_optional_f32(
            row,
            "reco_rho_mass_minus_gen_rho_mass",
            row_number,
        )?,
        reco_photon_pt_over_gen_photon_pt: parse_optional_f32(
            row,
            "reco_photon_pt_over_gen_photon_pt",
            row_number,
        )?,
        reco_rho_pt_over_gen_rho_pt: parse_optional_f32(
            row,
            "reco_rho_pt_over_gen_rho_pt",
            row_number,
        )?,
    })
}

fn parse_bool_field(
    row: &HashMap<&str, &str>,
    name: &str,
    row_number: usize,
) -> Result<bool, Box<dyn Error>> {
    let value = row
        .get(name)
        .ok_or_else(|| format!("row {row_number}: missing column {name}"))?;
    Ok(matches!(*value, "1" | "true" | "True"))
}

fn parse_optional_f32(
    row: &HashMap<&str, &str>,
    name: &str,
    row_number: usize,
) -> Result<f32, Box<dyn Error>> {
    let value = row
        .get(name)
        .ok_or_else(|| format!("row {row_number}: missing column {name}"))?;
    if value.is_empty() {
        Ok(f32::NAN)
    } else {
        value
            .parse::<f32>()
            .map_err(|_| format!("row {row_number}: failed to parse {name}").into())
    }
}

fn topology_code(value: &str) -> i32 {
    match value {
        "explicit_rho" => 1,
        "fallback_no_explicit_rho" => 2,
        "not_found" => 3,
        _ => 0,
    }
}

fn parse_field<T: std::str::FromStr>(
    row: &HashMap<&str, &str>,
    name: &str,
    row_number: usize,
) -> Result<T, Box<dyn Error>> {
    row.get(name)
        .ok_or_else(|| format!("row {row_number}: missing column {name}"))?
        .parse::<T>()
        .map_err(|_| format!("row {row_number}: failed to parse {name}").into())
}
