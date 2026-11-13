//! Renders every view of each bundled example, and of the views written by hand in
//! `tests/views/`, into the model's own `diagrams/` folder, as
//! `ortho render <model> -o <model>/diagrams` does, and removes the diagram of a view
//! that is gone.
use crate::{Result, write};
use ortho::parser::workspace::parse_sysml_files;
use ortho::pipeline::render_views;
use ortho::render::theme::LIGHT;

const EXAMPLES: &[&str] = &["examples/survey-drone", "examples/coffee-machine", "examples/web-shop", "tests/views"];

pub fn run() -> Result {
    for base in EXAMPLES {
        let mut files: Vec<String> = files_in(base)?.into_iter().filter(|name| name.ends_with(".sysml")).collect();
        files.sort();
        let files: Vec<String> = files.iter().map(|file| format!("{base}/{file}")).collect();
        let parsed = parse_sysml_files(&files)?;
        let rendering = render_views(&parsed.workspace, &LIGHT)?;
        let out = format!("{base}/diagrams");
        std::fs::create_dir_all(&out).map_err(|error| format!("{out}: {error}"))?;
        // A view that is gone leaves no diagram behind.
        for stale in files_in(&out)?.into_iter().filter(|name| name.ends_with(".svg")) {
            let path = format!("{out}/{stale}");
            std::fs::remove_file(&path).map_err(|error| format!("{path}: {error}"))?;
        }
        for rendered in &rendering.rendered {
            write(format!("{out}/{}", rendered.file), &rendered.result.svg)?;
            println!("Wrote {out}/{} ({})", rendered.file, rendered.result.summary);
            if let Some(warning) = &rendered.warning {
                println!("Warning: {warning}");
            }
        }
    }
    Ok(())
}

fn files_in(folder: &str) -> Result<Vec<String>> {
    let entries = std::fs::read_dir(folder).map_err(|error| format!("{folder}: {error}"))?;
    Ok(entries.filter_map(|entry| entry.ok()?.file_name().into_string().ok()).collect())
}
