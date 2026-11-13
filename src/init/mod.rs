//! `ortho init`: what a project needs to start, written from what ortho carries, so
//! it works with any editor and with none. The AI instructions go into the folder,
//! and with `--example` an example model beside them.
mod assets;
mod instructions;

use instructions::InstructionFile;
use std::path::Path;

/// The examples there are, by the name `--example` takes.
pub fn example_names() -> Vec<&'static str> {
    assets::EXAMPLES.iter().map(|example| example.name).collect()
}

/// Writes the AI instructions into `folder`, and copies the example named into a
/// folder of its own there. Returns what to tell the user, a line for each thing done.
/// Instructions that are there and differ are replaced only with `force`: they may be
/// the user's own.
pub fn init(folder: &Path, example: Option<&str>, force: bool) -> Result<Vec<String>, String> {
    let mut files = instructions::instruction_files(&assets::AI);
    // A project's own instructions for its agents are never written over.
    let theirs = instructions::AGENT_FILES.iter().find(|name| folder.join(name).exists());
    if theirs.is_some() {
        files.retain(|file| !file.unless_agent_file);
    }
    let differs = |file: &&InstructionFile| {
        std::fs::read_to_string(folder.join(&file.path)).is_ok_and(|there| there != file.content)
    };
    let changed: Vec<&str> = files.iter().filter(differs).map(|file| file.path.as_str()).collect();
    if !changed.is_empty() && !force {
        return Err(format!(
            "The AI instructions here differ from the ones ortho writes:\n  {}\nRun ortho init --force to replace them.",
            changed.join("\n  ")
        ));
    }
    let mut done = Vec::new();
    for file in &files {
        let to = folder.join(&file.path);
        if std::fs::read_to_string(&to).is_ok_and(|there| there == file.content) {
            done.push(format!("Up to date: {}", to.display()));
            continue;
        }
        write(&to, &file.content)?;
        done.push(format!("Wrote {}", to.display()));
    }
    if let Some(theirs) = theirs {
        done.push(format!(
            "Kept {}, which holds this project's instructions for agents",
            folder.join(theirs).display()
        ));
    }
    if let Some(name) = example {
        let example = assets::EXAMPLES
            .iter()
            .find(|example| example.name == name)
            .ok_or_else(|| format!("No example named '{name}'. There are: {}.", example_names().join(", ")))?;
        let to = folder.join(assets::free_name(example.name, |name| folder.join(name).exists()));
        for (file, text) in example.files {
            write(&to.join(file), text)?;
        }
        done.push(format!("Copied the {} example to {}", example.label.to_lowercase(), to.display()));
        done.push(format!("Start reading at {}", to.join(assets::FIRST_FILE).display()));
    }
    Ok(done)
}

fn write(path: &Path, content: &str) -> Result<(), String> {
    let error = |error: std::io::Error| format!("{}: {error}", path.display());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(error)?;
    }
    std::fs::write(path, content).map_err(error)
}
