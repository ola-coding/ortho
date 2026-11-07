//! The repository's own tasks, run from anywhere in it with `cargo xtask <task>`:
//!
//! - `render-examples`: every example's diagrams, and those of `tests/views/`, into
//!   its `diagrams/` folder.
//! - `deck`: the pitch deck, into `marketing/`.
//! - `omg-models`: OMG's SysML v2 models, into `.omg-models/`.
//! - `library-names`: the standard library's names, into `src/model/library.rs`.
//! - `vscode-grammar`: the VS Code extension's SysML v2 grammar, from ortho's keywords.
use std::path::Path;
use std::process::ExitCode;

mod deck;
mod library_names;
mod omg_models;
mod render_examples;
mod vscode_grammar;

type Result<T = ()> = std::result::Result<T, String>;

/// A task's name, what runs it, and what it does.
type Task = (&'static str, fn() -> Result, &'static str);

const TASKS: &[Task] = &[
    ("render-examples", render_examples::run, "render the diagrams of every example and of tests/views/"),
    ("deck", deck::run, "build the pitch deck into marketing/ from the committed diagrams"),
    ("omg-models", omg_models::run, "fetch OMG's SysML v2 models into .omg-models/"),
    ("library-names", library_names::run, "list the standard library's names into src/model/library.rs"),
    ("vscode-grammar", vscode_grammar::run, "write the VS Code extension's grammar from ortho's keyword list"),
];

fn main() -> ExitCode {
    // Every task reads and writes paths relative to the repository.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask/ is inside the repository");
    std::env::set_current_dir(root).expect("the repository is readable");
    let task = std::env::args().nth(1).unwrap_or_default();
    let Some((_, run, _)) = TASKS.iter().find(|(name, _, _)| *name == task) else {
        eprintln!("Usage: cargo xtask <task>\n\nTasks:");
        for (name, _, about) in TASKS {
            eprintln!("  {name:<16} {about}");
        }
        return ExitCode::FAILURE;
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("cargo xtask {task}: {message}");
            ExitCode::FAILURE
        }
    }
}

fn read(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn write(path: impl AsRef<Path>, text: &str) -> Result {
    let path = path.as_ref();
    std::fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))
}
