//! `ortho render`: every view a model declares, one SVG each.
use clap::{Parser, Subcommand};
use ortho::model::views::view_heading;
use ortho::parser::workspace::parse_sysml_files;
use ortho::pipeline::render_views;
use ortho::render::theme::{THEME_NAMES, Theme};
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "ortho", version, about = "Render diagrams from SysML v2 model files")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render every view the model declares, one SVG each
    Render {
        /// One or more .sysml files or folders, read as one model
        #[arg(required = true, value_name = "MODELS")]
        models: Vec<String>,
        /// The folder to write the SVGs to, each named after its view
        #[arg(short, long, value_name = "FOLDER")]
        out: String,
        /// Paint the diagrams with a theme
        #[arg(long, value_name = "NAME", default_value = "light", value_parser = THEME_NAMES)]
        theme: String,
    },
}

fn main() -> ExitCode {
    let Command::Render { models, out, theme } = Cli::parse().command;
    let theme = Theme::named(&theme).expect("clap admits only the themes there are");
    match render(&models, &out, theme) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// A file, or the .sysml files in a folder, by name.
fn expand_input(path: &str) -> Result<Vec<String>, String> {
    if !Path::new(path).is_dir() {
        return Ok(vec![path.to_string()]);
    }
    let entries = std::fs::read_dir(path).map_err(|error| format!("{path}: {error}"))?;
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.ends_with(".sysml"))
        .collect();
    names.sort();
    Ok(names.into_iter().map(|name| Path::new(path).join(name).to_string_lossy().into_owned()).collect())
}

fn render(models: &[String], out: &str, theme: &Theme) -> Result<(), String> {
    let mut files: Vec<String> = Vec::new();
    for model in models {
        for file in expand_input(model)? {
            let file = file.replace('\\', "/");
            if !files.contains(&file) {
                files.push(file);
            }
        }
    }
    if files.is_empty() {
        return Err("No .sysml files found in the given inputs.".to_string());
    }

    // A model that does not load is reported, not a crash.
    let parsed = parse_sysml_files(&files)?;
    for warning in &parsed.warnings {
        eprintln!("Warning: {warning}");
    }
    let workspace = &parsed.workspace;
    let rendering = render_views(workspace, theme)?;
    for skipped in &rendering.skipped {
        eprintln!("Skipped '{}': {}", view_heading(&workspace.ast, skipped.view), skipped.reason);
    }
    if rendering.rendered.is_empty() {
        return Err(if rendering.skipped.is_empty() {
            "The model declares no view. ortho draws the views a model declares, such as \
             view 'Functions' : GeneralView { expose Functions; }."
        } else {
            "The model declares no view ortho can draw."
        }
        .to_string());
    }
    std::fs::create_dir_all(out).map_err(|error| format!("{out}: {error}"))?;
    for rendered in &rendering.rendered {
        let path = Path::new(out).join(&rendered.file);
        std::fs::write(&path, &rendered.result.svg).map_err(|error| format!("{}: {error}", path.display()))?;
        println!("Wrote {} ({})", path.display(), rendered.result.summary);
    }
    Ok(())
}
