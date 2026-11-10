//! `ortho render`: every view a model declares, one SVG each, once or on every save;
//! `ortho lsp`, the same checks for an editor; and `ortho init`, what a project starts with.
use clap::{Parser, Subcommand};
use ortho::model::views::view_heading;
use ortho::parser::workspace::parse_sysml_files;
use ortho::pipeline::render_views;
use ortho::render::theme::{THEME_NAMES, Theme};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

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
        /// Keep running, and render again whenever a model file changes
        #[arg(short, long)]
        watch: bool,
    },
    /// Write AI instructions for Claude Code and GitHub Copilot into a folder, so an
    /// agent there knows the SysML v2 ortho reads and checks its work with ortho
    Init {
        /// The folder to write into
        #[arg(value_name = "FOLDER", default_value = ".")]
        folder: String,
        /// Also copy an example model into a folder of its own there
        #[arg(long, value_name = "NAME", value_parser = ortho::init::example_names())]
        example: Option<String>,
        /// Replace AI instructions that differ from the ones ortho writes
        #[arg(long)]
        force: bool,
    },
    /// Serve editors as a language server over stdin and stdout: the problems of each
    /// model folder as it is typed, go to definition, and diagrams on every save
    Lsp {
        /// Accepted and ignored: editors pass it to say the server talks over stdio
        #[arg(long, hide = true)]
        stdio: bool,
    },
}

fn main() -> ExitCode {
    let (models, out, theme, watching) = match Cli::parse().command {
        Command::Render { models, out, theme, watch } => (models, out, theme, watch),
        Command::Init { folder, example, force } => {
            return match ortho::init::init(Path::new(&folder), example.as_deref(), force) {
                Ok(done) => {
                    for line in done {
                        println!("{line}");
                    }
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::FAILURE
                }
            };
        }
        Command::Lsp { .. } => {
            let stdin = std::io::stdin().lock();
            return match ortho::lsp::serve(stdin, std::io::stdout().lock()) {
                Ok(true) => ExitCode::SUCCESS,
                Ok(false) => ExitCode::FAILURE,
                Err(error) => {
                    eprintln!("ortho lsp: {error}");
                    ExitCode::FAILURE
                }
            };
        }
    };
    let theme = Theme::named(&theme).expect("clap admits only the themes there are");
    if watching {
        watch(&models, &out, theme);
    }
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

/// The .sysml files the inputs name, each once, with forward slashes.
fn model_files(models: &[String]) -> Result<Vec<String>, String> {
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
    Ok(files)
}

fn render(models: &[String], out: &str, theme: &Theme) -> Result<(), String> {
    let files = model_files(models)?;

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
        if let Some(warning) = &rendered.warning {
            eprintln!("Warning: {warning}");
        }
    }
    Ok(())
}

/// How often `--watch` looks at the model files.
const POLL: Duration = Duration::from_millis(250);

/// What a model file looked like when last seen: when it was changed and how long it
/// was, or nothing while it cannot be read, as during an editor's save.
type Seen = Vec<(String, Option<(SystemTime, u64)>)>;

/// The model files the inputs name now, so a file added to a folder is seen too.
fn look(models: &[String]) -> Seen {
    let files = model_files(models).unwrap_or_default();
    files
        .into_iter()
        .map(|file| {
            let state = std::fs::metadata(&file).ok().and_then(|meta| Some((meta.modified().ok()?, meta.len())));
            (file, state)
        })
        .collect()
}

/// Renders, then renders again on every change to the model files, until stopped
/// with Ctrl+C. Files are looked at rather than subscribed to, which works the same
/// on every system and network drive, with no dependency. A model that does not load
/// is reported and waited on, not the end: the next save may fix it.
fn watch(models: &[String], out: &str, theme: &Theme) -> ! {
    let mut seen = look(models);
    loop {
        if let Err(message) = render(models, out, theme) {
            eprintln!("{message}");
        }
        println!("Watching {} file(s) for changes. Press Ctrl+C to stop.", seen.len());
        // Waits for a change, then for the files to settle, since an editor may save
        // in more than one write.
        loop {
            std::thread::sleep(POLL);
            let now = look(models);
            if now != seen {
                seen = now;
                break;
            }
        }
        loop {
            std::thread::sleep(POLL);
            let now = look(models);
            if now == seen {
                break;
            }
            seen = now;
        }
        println!();
    }
}
