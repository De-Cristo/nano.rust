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
}

impl CandidateTable {
    fn read_csv(path: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let mut lines = content.lines();
        let header = lines
            .next()
            .ok_or_else(|| format!("candidate CSV is empty: {}", path.display()))?;
        validate_header(header)?;

        let mut table = Self::default();
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
    }

    fn write_root(mut self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
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
}

fn validate_header(header: &str) -> Result<(), Box<dyn Error>> {
    let columns = header.split(',').collect::<Vec<_>>();
    if columns == HEADER {
        Ok(())
    } else {
        Err(format!("unexpected candidate CSV header: {header}").into())
    }
}

fn parse_row(line: &str, row_number: usize) -> Result<CsvRow, Box<dyn Error>> {
    let values = line.split(',').collect::<Vec<_>>();
    if values.len() != HEADER.len() {
        return Err(format!(
            "row {row_number}: expected {} columns, found {}",
            HEADER.len(),
            values.len()
        )
        .into());
    }
    let map = HEADER
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
    })
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
