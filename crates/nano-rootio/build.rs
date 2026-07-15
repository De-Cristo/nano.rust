use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    if env::var_os("CARGO_FEATURE_XROOTD").is_none() {
        return;
    }

    println!("cargo:rerun-if-changed=src/xrootd_shim.cc");
    println!("cargo:rerun-if-env-changed=XROOTD_INCLUDE_DIR");
    println!("cargo:rerun-if-env-changed=XROOTD_LIB_DIR");
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=AR");

    let include_dir = include_dir();
    let lib_dir = lib_dir();
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("Cargo must set OUT_DIR"));
    let object = out_dir.join("xrootd_shim.o");
    let archive = out_dir.join("libnano_rootio_xrootd_shim.a");
    let compiler = env::var("CXX").unwrap_or_else(|_| "c++".to_string());
    let archiver = env::var("AR").unwrap_or_else(|_| "ar".to_string());

    run(
        Command::new(compiler)
            .arg("-std=c++17")
            .arg("-fPIC")
            .arg("-c")
            .arg("src/xrootd_shim.cc")
            .arg("-I")
            .arg(&include_dir)
            .arg("-o")
            .arg(&object),
        "compile XRootD bridge",
    );
    run(
        Command::new(archiver)
            .arg("crus")
            .arg(&archive)
            .arg(&object),
        "archive XRootD bridge",
    );

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=nano_rootio_xrootd_shim");
    println!("cargo:rustc-link-lib=dylib=XrdCl");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}

fn include_dir() -> PathBuf {
    find_dir(
        "XROOTD_INCLUDE_DIR",
        &["/usr/include/xrootd", "/usr/include"],
        "XrdCl/XrdClFile.hh",
        "XRootD headers",
    )
}

fn lib_dir() -> PathBuf {
    find_dir(
        "XROOTD_LIB_DIR",
        &["/usr/lib64", "/usr/lib/x86_64-linux-gnu", "/usr/lib"],
        "libXrdCl.so",
        "libXrdCl.so",
    )
}

fn find_dir(env_var: &str, defaults: &[&str], required_file: &str, description: &str) -> PathBuf {
    if let Some(path) = env::var_os(env_var).map(PathBuf::from) {
        if path.join(required_file).is_file() {
            return path;
        }
        panic!(
            "{description} were not found in {}. Install xrootd-client-devel and xrootd-devel, or set {env_var} to the directory containing {required_file}.",
            path.display()
        );
    }

    defaults
        .iter()
        .map(PathBuf::from)
        .find(|path| path.join(required_file).is_file())
        .unwrap_or_else(|| {
            panic!(
                "{description} were not found. Checked {}. Install xrootd-client-devel and xrootd-devel, or set {env_var} to the directory containing {required_file}.",
                defaults.join(", ")
            )
        })
}

fn run(command: &mut Command, action: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to {action}: {error}"));
    if !status.success() {
        panic!("failed to {action}: {status}");
    }
}
