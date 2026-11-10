//! The AI instructions `ortho init` writes into a project: a skill for Claude Code and
//! instructions for GitHub Copilot, each telling an agent how to write a model ortho
//! draws and to check its work with ortho. The same as the VS Code extension writes
//! (`editors/vscode/src/instructions.js`), but for where ortho is: they name no path
//! of this machine, since they are committed and shared.

const SKILL: &str = "sysml-v2-ortho";

/// What an agent does with a model, in either tool. `language` says where the language
/// reference is, as the sentence's object.
fn workflow(language: &str) -> String {
    format!(
        r#"The `.sysml` files in a folder are one SysML v2 model. ortho reads them together and draws, as SVG, every view the model declares. {language} says what ortho reads and what each view draws from it. Read it before you change a model, and write only what it describes.

## Check your work with ortho

After every change, render the model's folder and read everything ortho prints:

```sh
ortho render <model folder> -o <a temporary folder>
```

Write the diagrams into the model's own folder only when asked to update them.

- **Errors**, under `Failed to parse:`, are what SysML v2 forbids, or a name that resolves to nothing. ortho draws nothing until they are fixed.
- **Warnings**, each starting `Warning:`, are views of the model that disagree: a function no part or module performs, a use case no function tree refines, software that no deployment allocates to a host. The model draws, but it is not finished.

Each problem names its file and line. Fix every error and every warning before you finish. If you leave a warning, say which one and why.

## Fix the cause, not the line

Never delete or comment out the line a problem points at to make it go away. A `perform`, a `#refinement dependency` or an `allocate` that no longer resolves means a name changed: find the new name, which the error suggests in a chain such as `beverage.prepareCoffee.storeBeans`, and update the reference. A deleted line leaves a function with nothing to realize it, or a use case nothing details.

When you rename an element, update every reference to it in every file of the folder, then render again.

## Where ortho is

Run `ortho`. If it is not found, try `~/.cargo/bin/ortho`. To install it: `cargo install ortho`, or download it from https://github.com/ola-coding/ortho/releases.
"#
    )
}

/// The language reference, a level deeper, to follow the instructions in one file: its
/// title becomes a section. A heading inside a code block is SysML, and stays.
fn nested(language: &str) -> String {
    let mut fenced = false;
    let lines = language.split('\n').map(|line| {
        if line.starts_with("```") {
            fenced = !fenced;
        }
        let heading = line.find(' ').is_some_and(|at| (1..=5).contains(&at) && line[..at].bytes().all(|b| b == b'#'));
        if !fenced && heading { format!("#{line}") } else { line.to_string() }
    });
    lines.collect::<Vec<_>>().join("\n")
}

/// The files to write, by path from the project's folder, from ortho's LANGUAGE.md.
pub fn instruction_files(language: &str) -> Vec<(String, String)> {
    let skill = format!(
        "---\nname: {SKILL}\ndescription: Write and change SysML v2 models, .sysml files, that ortho draws as architecture views, and check them with ortho. Use whenever you create or edit a .sysml file.\n---\n\n# SysML v2 models that ortho draws\n\n{}",
        workflow("[LANGUAGE.md](LANGUAGE.md), beside this file,")
    );
    let copilot = format!(
        "---\napplyTo: \"**/*.sysml\"\n---\n\n# SysML v2 models that ortho draws\n\n{}\n{}",
        workflow("The language reference below"),
        nested(language)
    );
    vec![
        (format!(".claude/skills/{SKILL}/SKILL.md"), skill),
        (format!(".claude/skills/{SKILL}/LANGUAGE.md"), language.to_string()),
        (".github/instructions/sysml-v2.instructions.md".to_string(), copilot),
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
    fn writes_a_skill_its_reference_and_copilot_instructions() {
        let files = instruction_files("# L");
        assert_eq!(files.len(), 3);
        assert!(files[0].1.contains("ortho render <model folder>"));
        assert_eq!(files[1].1, "# L");
        assert!(files[2].1.ends_with("## L"));
    }
}
