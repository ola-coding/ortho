//! `ortho render`, run as a user runs it.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;
use common::repository;

/// Runs ortho in the repository's folder, as the examples are rendered.
fn ortho(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ortho")).args(args).current_dir(repository()).output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// An empty folder of the test's own.
fn scratch(name: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli").join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// A folder holding one model file of `text`.
fn model(name: &str, text: &str) -> PathBuf {
    let folder = scratch(name);
    std::fs::write(folder.join("model.sysml"), text).unwrap();
    folder
}

fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}

fn files_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(folder).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    names
}

#[test]
fn says_its_version() {
    let output = ortho(&["--version"]);
    assert!(output.status.success());
    assert_eq!(stdout(&output).trim(), format!("ortho {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn renders_the_examples_as_their_committed_diagrams() {
    // The web shop has no hardware, so no physical or deployment view.
    for (example, views) in [("coffee-machine", 6), ("survey-drone", 6), ("web-shop", 4)] {
        let out = scratch(example);
        let output = ortho(&["render", &format!("examples/{example}"), "-o", path(&out)]);
        assert!(output.status.success(), "{}", stderr(&output));
        // Every example fits landscape A4, so ortho warns of none.
        assert_eq!(stderr(&output).lines().collect::<Vec<_>>(), Vec::<&str>::new(), "{example}");
        let committed = repository().join("examples").join(example).join("diagrams");
        let written = files_in(&out);
        assert_eq!(written, files_in(&committed));
        for file in &written {
            // As Git on Windows may check it out.
            let expected = std::fs::read_to_string(committed.join(file)).unwrap().replace("\r\n", "\n");
            let actual = std::fs::read_to_string(out.join(file)).unwrap();
            assert!(actual == expected, "{example}: {file} is not as committed");
        }
        // One line for each file, saying what it holds.
        let said: Vec<String> = stdout(&output).lines().map(String::from).collect();
        assert_eq!(said.len(), views, "{example}");
        let logical = out.join(format!("logical-view-{example}.svg"));
        assert!(said.iter().any(|line| line.starts_with(&format!("Wrote {} (", logical.display()))), "{said:?}");
        assert!(said.iter().any(|line| line.ends_with("message(s))")), "{said:?}");
    }
}

#[test]
fn reads_a_file_named_twice_or_inside_a_folder_named_too_once() {
    let out = scratch("twice");
    let output = ortho(&[
        "render",
        "examples/coffee-machine",
        "examples/coffee-machine/use-case.sysml",
        "examples/coffee-machine/use-case.sysml",
        "-o",
        path(&out),
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(files_in(&out).len(), 6);
}

#[test]
fn paints_a_theme_by_name() {
    let out = scratch("dark");
    let output =
        ortho(&["render", "examples/coffee-machine/use-case.sysml", "--theme", "dark-glass", "-o", path(&out)]);
    assert!(output.status.success(), "{}", stderr(&output));
    let svg = std::fs::read_to_string(out.join("use-case-view-coffee-machine.svg")).unwrap();
    assert!(svg.contains(r#"id="gloss""#));

    let output = ortho(&["render", "examples/coffee-machine", "--theme", "sepia", "-o", path(&out)]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("light, dark-glass"), "{}", stderr(&output));
}

#[test]
fn needs_a_model_and_a_folder_to_write_to() {
    assert!(!ortho(&["render", "-o", "out"]).status.success());
    let output = ortho(&["render", "examples/coffee-machine"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("--out"), "{}", stderr(&output));
}

#[test]
fn says_when_a_folder_holds_no_model() {
    let empty = scratch("empty");
    let output = ortho(&["render", path(&empty), "-o", path(&scratch("empty-out"))]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "No .sysml files found in the given inputs.\n");
}

#[test]
fn says_which_file_cannot_be_read() {
    let output = ortho(&["render", "examples/no-such-model.sysml", "-o", path(&scratch("unread"))]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).starts_with("examples/no-such-model.sysml: "), "{}", stderr(&output));
}

#[test]
fn reports_a_model_that_does_not_load_and_writes_nothing() {
    let folder = model("broken", "package P {\n    part def A;\n    part a : Missing;\n}\n");
    let out = folder.join("diagrams");
    let output = ortho(&["render", path(&folder), "-o", path(&out)]);
    assert_eq!(output.status.code(), Some(1));
    let file = folder.join("model.sysml").to_string_lossy().replace('\\', "/");
    assert_eq!(
        stderr(&output),
        format!("Failed to parse:\n  {file} line 3: Could not resolve reference to Definition named 'Missing'.\n")
    );
    assert!(!out.exists());
}

#[test]
fn warns_and_still_renders() {
    let folder =
        model("warned", "package ScalarValues {\n    action def A;\n    view v : GeneralView { expose A; }\n}\n");
    let out = folder.join("diagrams");
    let output = ortho(&["render", path(&folder), "-o", path(&out)]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).starts_with("Warning: "), "{}", stderr(&output));
    assert_eq!(files_in(&out), ["v.svg"]);
}

#[test]
fn says_why_it_draws_nothing() {
    let none = model("no-view", "package P { action def A; }");
    let output = ortho(&["render", path(&none), "-o", path(&none.join("diagrams"))]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).starts_with("The model declares no view."), "{}", stderr(&output));

    let undrawable = model("undrawable", "package P { action def A; view states : StateTransitionView { expose A; } }");
    let output = ortho(&["render", path(&undrawable), "-o", path(&undrawable.join("diagrams"))]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "Skipped 'states': ortho draws a GeneralView, an InterconnectionView or a SequenceView, not \
         'StateTransitionView'.\nThe model declares no view ortho can draw.\n"
    );
}

#[test]
fn refuses_two_views_for_one_file() {
    let folder = model(
        "same-file",
        "package P { action def A; view 'Functions' : GeneralView { expose A; } view functions : GeneralView { expose A; } }",
    );
    let output = ortho(&["render", path(&folder), "-o", path(&folder.join("diagrams"))]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).starts_with("'Functions' and 'functions' would both be written to functions.svg"),
        "{}",
        stderr(&output)
    );
}

/// ortho run with `--watch` on `folder`, writing to its `diagrams/`, stopped when dropped.
struct Watching(std::process::Child);

impl Watching {
    fn start(folder: &Path) -> Watching {
        let child = Command::new(env!("CARGO_BIN_EXE_ortho"))
            .args(["render", path(folder), "-o", path(&folder.join("diagrams")), "--watch"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        Watching(child)
    }
}

impl Drop for Watching {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The file once it holds what `ready` looks for, waiting up to ten seconds.
fn eventually(file: &Path, ready: impl Fn(&str) -> bool) -> String {
    for _ in 0..100 {
        if let Ok(text) = std::fs::read_to_string(file)
            && ready(&text)
        {
            return text;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("{} was not written as expected", file.display());
}

const FUNCTIONS: &str = "package P { action def Brew; view functions : GeneralView { expose Brew; } }";

#[test]
fn watches_and_renders_again_on_every_save() {
    let folder = model("watch", FUNCTIONS);
    let _watching = Watching::start(&folder);
    let svg = folder.join("diagrams").join("functions.svg");
    eventually(&svg, |text| text.contains("Brew"));

    std::fs::write(folder.join("model.sysml"), FUNCTIONS.replace("Brew", "Grind")).unwrap();
    eventually(&svg, |text| text.contains("Grind") && !text.contains("Brew"));

    // A file added to the folder is part of the model from then on.
    let more = "package Q { action def Heat; view heating : GeneralView { expose Heat; } }";
    std::fs::write(folder.join("more.sysml"), more).unwrap();
    eventually(&folder.join("diagrams").join("heating.svg"), |text| text.contains("Heat"));
}

#[test]
fn watching_outlasts_a_model_that_does_not_load() {
    let folder = model("watch-broken", "package P { part a : Missing; }");
    let _watching = Watching::start(&folder);
    std::thread::sleep(std::time::Duration::from_millis(600));
    std::fs::write(folder.join("model.sysml"), FUNCTIONS).unwrap();
    eventually(&folder.join("diagrams").join("functions.svg"), |text| text.contains("Brew"));
}
