//! The files ortho hands to AI agents, in `ai/`, say only what is true of ortho.
use ortho::parser::workspace::{Severity, Workspace};
use std::path::Path;

/// Everything ortho reports for a model on its own: what stops it parsing, else its
/// errors and warnings.
fn reports(text: &str) -> Vec<String> {
    match Workspace::build(vec![("model.sysml".to_string(), text.to_string())]) {
        Err(failures) => failures,
        Ok(workspace) => {
            let diagnostics = workspace.documents.iter().flat_map(|document| &document.diagnostics);
            let reported = diagnostics.filter(|d| matches!(d.severity, Severity::Error | Severity::Warning));
            reported.map(|d| d.message.clone()).collect()
        }
    }
}

/// The text of the first code block opened by `fence` followed by a line end.
fn block<'a>(section: &'a str, fence: &str) -> Option<&'a str> {
    let start = section.find(&format!("{fence}\n"))? + fence.len() + 1;
    let length = section[start..].find("```")?;
    Some(&section[start..start + length])
}

// Each mistake shows the wrong shape, what ortho prints for it, and the right shape.
// An agent is told to trust that message, so it has to be the one ortho prints.
#[test]
fn each_mistake_draws_the_message_it_shows_and_its_fix_loads_cleanly() {
    let doc = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("ai/MISTAKES.md")).unwrap();
    let mistakes: Vec<&str> = doc.split("\n### ").skip(1).collect();
    assert!(mistakes.len() >= 10, "{} mistakes", mistakes.len());
    for mistake in mistakes {
        let title = mistake.lines().next().unwrap();
        let wrong = block(mistake, "```sysml wrong").unwrap_or_else(|| panic!("{title}: no wrong shape"));
        let right = block(mistake, "```sysml").unwrap_or_else(|| panic!("{title}: no right shape"));
        assert_eq!(reports(right), Vec::<String>::new(), "{title}: the right shape");
        let wrong_reports = reports(wrong);
        if mistake.contains("\northo prints nothing.\n") {
            assert_eq!(wrong_reports, Vec::<String>::new(), "{title}: the wrong shape");
        } else {
            let line = mistake.lines().find_map(|line| line.strip_prefix("ortho prints: `"));
            let message = line.and_then(|line| line.strip_suffix("`")).unwrap_or_else(|| panic!("{title}: no message"));
            assert!(wrong_reports.iter().any(|report| report.ends_with(message)), "{title}: {wrong_reports:?}");
        }
    }
}
