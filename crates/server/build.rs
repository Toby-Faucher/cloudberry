//! Sets `CLOUDBERRY_BUILD`, the build id in `welcome` and `/api/version`. First that is set: the
//! `CLOUDBERRY_BUILD_ID` variable at build time, the git commit, else the package version.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=CLOUDBERRY_BUILD_ID");
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    // The reflog changes on every commit and checkout, so a new commit reruns this.
    if let Some(log) = git(&[
        "rev-parse",
        "--path-format=absolute",
        "--git-path",
        "logs/HEAD",
    ]) {
        println!("cargo:rerun-if-changed={log}");
    }
    let id = std::env::var("CLOUDBERRY_BUILD_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| git(&["rev-parse", "--short", "HEAD"]))
        .unwrap_or_else(|| std::env::var("CARGO_PKG_VERSION").unwrap_or_default());
    println!("cargo:rustc-env=CLOUDBERRY_BUILD={id}");
}
