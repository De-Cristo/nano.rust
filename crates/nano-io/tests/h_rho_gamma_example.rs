use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn h_rho_gamma_example_runs_on_local_file_if_present() {
    let input = match env::var("NANO_H_RHO_GAMMA_FILE") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("SKIP: NANO_H_RHO_GAMMA_FILE not set");
            return;
        }
    };
    if !Path::new(&input).exists() {
        eprintln!("SKIP: {input} absent");
        return;
    }
    let csv_path = env::temp_dir().join(format!(
        "h_rho_gamma_example_{}_{}.csv",
        std::process::id(),
        "candidates"
    ));
    let _ = fs::remove_file(&csv_path);
    let csv_arg = csv_path
        .to_str()
        .expect("temporary CSV path should be UTF-8")
        .to_string();

    let output = Command::new(option_env!("CARGO").unwrap_or("cargo"))
        .args([
            "run",
            "-p",
            "nano-io",
            "--example",
            "h_rho_gamma",
            "--quiet",
            "--",
            &input,
            "100",
            "--csv",
            &csv_arg,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run h_rho_gamma example");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "example failed\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout,
        stderr
    );
    assert!(stdout.contains("input: "), "stdout:\n{stdout}");
    assert!(
        stdout.contains("cut_source: configs/h_rho_gamma.toml [baseline.zcountinghlt_naive]"),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains("branch_schema: ok"), "stdout:\n{stdout}");
    assert!(stdout.contains("cutflow:"), "stdout:\n{stdout}");
    assert!(stdout.contains("processed_events:"), "stdout:\n{stdout}");
    assert!(stdout.contains("pfcand_pions:"), "stdout:\n{stdout}");
    assert!(stdout.contains("candidate_output: "), "stdout:\n{stdout}");

    let csv = fs::read_to_string(&csv_path).expect("read candidate CSV");
    let mut lines = csv.lines();
    let header = lines.next().expect("CSV header");
    assert!(header.contains("run,luminosityBlock,event"), "CSV:\n{csv}");
    assert!(header.contains("photon_pt"), "CSV:\n{csv}");
    assert!(header.contains("rho_pt_over_photon_pt"), "CSV:\n{csv}");
    if input.ends_with("48f9579f-2a00-469f-864e-4bc4f064984e.root") {
        assert!(
            lines.count() > 0,
            "expected candidate rows in known local file"
        );
    }
    let _ = fs::remove_file(csv_path);
}
