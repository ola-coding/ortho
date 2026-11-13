//! The AI instructions `ortho init` writes into a project: a skill for Claude Code,
//! instructions for GitHub Copilot, and an AGENTS.md for every other agent, each
//! telling an agent how to write a model ortho draws and to check its work with ortho.
//! What they say is written once, in ortho's `ai/` folder and its LANGUAGE.md and
//! VIEWS.md, which ortho carries; only how they are put together is here, as the VS
//! Code extension does it (`editors/vscode/src/instructions.js`), but for where ortho
//! is: they name no path of this machine, since they are committed and shared.
use super::assets::Sources;

const SKILL: &str = "sysml-v2-ortho";

/// When the skill is used: whenever an agent works on a model, or is asked for one.
const DESCRIPTION: &str = "Write, change and review SysML v2 models, .sysml files, that ortho draws as architecture views, and check them with ortho. \
Use whenever you create, edit or review a .sysml file, or are asked to model a system in SysML v2: \
its use cases, functions, software, hardware or deployment.";

/// Files that, when a project has one, already hold its agents' instructions. Zed
/// reads the first of them it finds, so an AGENTS.md written beside a CLAUDE.md would
/// hide it there: AGENTS.md is written only when none is there.
pub const AGENT_FILES: [&str; 4] = ["AGENTS.md", "AGENT.md", "CLAUDE.md", "GEMINI.md"];

/// Where ortho is, from an agent in any editor, and what it offers a project.
const WHERE: &str = "
## Where ortho is

Run `ortho`. If it is not found, try `~/.cargo/bin/ortho`. To install it: `cargo install ortho`, or download it from https://github.com/ola-coding/ortho/releases.

`ortho init --example <name>` copies an example model into the project; `ortho init --help` names them.
";

/// A file to write, by path from the project's folder.
pub struct InstructionFile {
    pub path: String,
    pub content: String,
    /// Written only if none of `AGENT_FILES` is in the project, and never replaced.
    pub unless_agent_file: bool,
}

/// A copied file without the comment that says where it was copied from.
fn body(text: &str) -> &str {
    text.strip_prefix("<!--").and_then(|rest| rest.split_once("-->\n\n")).map_or(text, |(_, body)| body)
}

/// A reference, a level deeper, to follow the instructions in one file: its title
/// becomes a section. A heading inside a code block is SysML, and stays.
fn nested(text: &str) -> String {
    let mut fenced = false;
    let lines = text.split('\n').map(|line| {
        if line.starts_with("```") {
            fenced = !fenced;
        }
        let heading = line.find(' ').is_some_and(|at| (1..=5).contains(&at) && line[..at].bytes().all(|b| b == b'#'));
        if !fenced && heading { format!("#{line}") } else { line.to_string() }
    });
    lines.collect::<Vec<_>>().join("\n")
}

/// Text written outside the skill's folder: a link to one of the files beside the
/// skill points into that folder, from `.github/instructions/`.
fn from_instructions(text: &str) -> String {
    let mut text = text.to_string();
    for name in ["WORKFLOW.md", "LANGUAGE.md", "VIEWS.md", "MISTAKES.md"] {
        for end in [')', '#'] {
            text = text.replace(&format!("]({name}{end}"), &format!("](../../.claude/skills/{SKILL}/{name}{end}"));
        }
    }
    text
}

/// The files to write, from what ortho carries.
pub fn instruction_files(ai: &Sources) -> Vec<InstructionFile> {
    let workflow = body(ai.workflow);
    let folder = format!(".claude/skills/{SKILL}");
    let skill = format!(
        "---\nname: {SKILL}\ndescription: {DESCRIPTION}\n---\n\n# SysML v2 models that ortho draws\n\n{workflow}{WHERE}"
    );
    let copilot = format!(
        "---\napplyTo: \"**/*.sysml\"\n---\n\n# SysML v2 models that ortho draws\n\n{}{WHERE}\n{}\n{}",
        from_instructions(workflow),
        nested(&from_instructions(body(ai.language))),
        nested(&from_instructions(body(ai.mistakes)))
    );
    let file =
        |path: String, content: &str| InstructionFile { path, content: content.to_string(), unless_agent_file: false };
    vec![
        file(format!("{folder}/SKILL.md"), &skill),
        file(format!("{folder}/LANGUAGE.md"), ai.language),
        file(format!("{folder}/VIEWS.md"), ai.views),
        file(format!("{folder}/MISTAKES.md"), ai.mistakes),
        file(".github/instructions/sysml-v2.instructions.md".to_string(), &copilot),
        InstructionFile {
            path: "AGENTS.md".to_string(),
            content: body(ai.agents).to_string(),
            unless_agent_file: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_go_a_level_deeper_outside_code() {
        let text = "# Title\n## Part\n```sysml\n# not a heading\n```\n#tag";
        assert_eq!(nested(text), "## Title\n### Part\n```sysml\n# not a heading\n```\n#tag");
    }

    #[test]
    fn puts_the_files_together_from_their_sources() {
        let ai = Sources {
            language: "<!-- Copied. -->\n\n# L\nsee [V](VIEWS.md#x)",
            views: "# V",
            workflow: "<!-- Copied. -->\n\nRead [L](LANGUAGE.md).\n",
            mistakes: "# M",
            agents: "<!-- Copied. -->\n\n# A",
        };
        let files = instruction_files(&ai);
        let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths[0], ".claude/skills/sysml-v2-ortho/SKILL.md");
        assert_eq!(paths[4..], [".github/instructions/sysml-v2.instructions.md", "AGENTS.md"]);
        assert!(files[0].content.contains("\n\nRead [L](LANGUAGE.md).\n\n## Where ortho is"));
        assert_eq!(files[1].content, ai.language);
        assert!(files[4].content.contains("Read [L](../../.claude/skills/sysml-v2-ortho/LANGUAGE.md)."));
        assert!(files[4].content.contains("## L\nsee [V](../../.claude/skills/sysml-v2-ortho/VIEWS.md#x)\n## M"));
        assert_eq!(files[5].content, "# A");
        assert!(files[5].unless_agent_file && !files[4].unless_agent_file);
    }
}
