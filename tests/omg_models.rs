//! OMG's own SysML v2 models, fetched by `cargo xtask omg-models`. ortho should load every
//! one of them; this counts how many it does, so the count only moves on purpose.
use ortho::parser::workspace::{Severity, Workspace};
use std::path::{Path, PathBuf};

mod common;

const LOADED: usize = 252;

fn sysml_files(folder: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(folder).unwrap().map(|entry| entry.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sysml_files(&path, files);
        } else if path.extension().is_some_and(|e| e == "sysml") {
            files.push(path);
        }
    }
}

#[test]
#[ignore = "needs cargo xtask omg-models"]
fn omg_models_load_as_many_as_counted() {
    // One model, as OMG's pilot implementation holds them: the training models refer
    // to one another across folders.
    let root = common::repository().join(".omg-models/sysml/src");
    let mut files = Vec::new();
    for folder in ["examples", "training", "validation"] {
        sysml_files(&root.join(folder), &mut files);
    }
    let sources = files.iter().map(|path| {
        (path.to_string_lossy().into_owned(), String::from_utf8_lossy(&std::fs::read(path).unwrap()).into_owned())
    });
    let workspace = Workspace::build(sources.collect()).unwrap_or_else(|failures| panic!("{failures:?}"));
    let loaded = workspace
        .documents
        .iter()
        .filter(|document| !document.diagnostics.iter().any(|d| d.severity == Severity::Error))
        .count();
    assert_eq!(loaded, LOADED, "{loaded} of {} load: update LOADED", files.len());
}
