//! Fetches OMG's SysML v2 models into `.omg-models/`, at the release the tests count
//! against, with the standard library `cargo xtask library-names` reads. Needs Git.
//!
//! They are the example, training and validation models of OMG's SysML-v2-Release.
//! They are EPL-2.0, so they are fetched, not committed.
use crate::Result;
use std::path::Path;
use std::process::Command;

const DIR: &str = ".omg-models";
const REPO: &str = "https://github.com/Systems-Modeling/SysML-v2-Release.git";
/// The 2026-09 release.
const COMMIT: &str = "b7a6b77000c9136f8f24822fc9d2cb237c76267f";
const FOLDERS: &[&str] = &["sysml/src/examples", "sysml/src/training", "sysml/src/validation", "sysml.library"];

fn git(args: &[&str]) -> Result<String> {
    let output = Command::new("git").args(["-C", DIR]).args(args).output().map_err(|error| format!("git: {error}"))?;
    if !output.status.success() {
        return Err(format!("git {}: {}", args.join(" "), String::from_utf8_lossy(&output.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn run() -> Result {
    let sparse: Vec<&str> = ["sparse-checkout", "set"].iter().chain(FOLDERS).copied().collect();
    let fetched = Path::new(DIR).exists().then(|| git(&["rev-parse", "HEAD"]).ok()).flatten();
    if fetched.as_deref() == Some(COMMIT) {
        git(&sparse)?;
        println!("{DIR}/ holds OMG's models at {}", &COMMIT[..7]);
        return Ok(());
    }
    if Path::new(DIR).exists() {
        std::fs::remove_dir_all(DIR).map_err(|error| format!("{DIR}: {error}"))?;
    }
    std::fs::create_dir(DIR).map_err(|error| format!("{DIR}: {error}"))?;
    git(&["init", "-q"])?;
    git(&["remote", "add", "origin", REPO])?;
    git(&sparse)?;
    git(&["fetch", "-q", "--depth", "1", "--filter=blob:none", "origin", COMMIT])?;
    git(&["checkout", "-q", "FETCH_HEAD"])?;
    println!("Fetched OMG's models at {} into {DIR}/", &COMMIT[..7]);
    Ok(())
}
