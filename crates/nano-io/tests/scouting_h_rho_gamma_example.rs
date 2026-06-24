use std::env;
use std::path::Path;
use std::process::Command;

#[test]
fn scouting_h_rho_gamma_example_runs_on_local_file_if_present() {
    let input = match env::var("NANO_SCOUTING_HRHOGAMMA_FILE") {
        Ok(value) => value,
        Err(_) => {
            eprintln!("SKIP: NANO_SCOUTING_HRHOGAMMA_FILE not set");
            return;
        }
    };
    if !Path::new(&input).exists() {
        eprintln!("SKIP: {input} absent");
        return;
    }

    let output = Command::new(option_env!("CARGO").unwrap_or("cargo"))
        .args([
            "run",
            "-p",
            "nano-io",
            "--example",
            "scouting_h_rho_gamma",
            "--quiet",
            "--",
            &input,
            "50",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run scouting_h_rho_gamma example");

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
    assert!(stdout.contains("branch_schema: ok"), "stdout:\n{stdout}");
    assert!(stdout.contains("cutflow:"), "stdout:\n{stdout}");
    assert!(stdout.contains("processed_events:"), "stdout:\n{stdout}");
    assert!(stdout.contains("pfcand_pions:"), "stdout:\n{stdout}");
}
