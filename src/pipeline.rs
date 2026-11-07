//! A view, from what it exposes to its SVG.
use crate::diagrams::deployment::extract_deployment_graph;
use crate::diagrams::implementation::extract_implementation_graph;
use crate::diagrams::logical::extract_logical_graph;
use crate::diagrams::physical::extract_physical_graph;
use crate::diagrams::sequence::extract_sequence_model;
use crate::diagrams::use_case::extract_use_case_graph;
use crate::layout::SourceMark;
use crate::layout::elk::{Algorithm, LayoutOptions, layout_graph};
use crate::layout::tree::layout_tree;
use crate::layout::use_case::layout_use_cases;
use crate::model::graph::DiagramGraph;
use crate::model::views::{
    DiagramKind, declared_views, diagram_kind, exposed_elements, view_file_name, view_heading, why_undrawable,
};
use crate::parser::ast::{NodeId, Prop};
use crate::parser::workspace::Workspace;
use crate::render::page::{page_warning, svg_size};
use crate::render::sequence::render_sequence_svg;
use crate::render::svg::{DiagramTitle, render_svg};
use crate::render::theme::Theme;
use std::path::{Component, Path, PathBuf};

pub struct RenderResult {
    pub svg: String,
    pub summary: String,
}

/// Renders what a view of `kind` exposing `exposed` shows, or why ELK could not lay it out.
pub fn render_exposed(
    workspace: &Workspace,
    kind: DiagramKind,
    exposed: &[NodeId],
    title: &DiagramTitle,
    theme: &Theme,
) -> Result<RenderResult, String> {
    let ast = &workspace.ast;
    let elk = |graph: DiagramGraph, options: LayoutOptions| -> Result<_, String> {
        let laid_out = layout_graph(&graph, options)?;
        Ok((graph, laid_out))
    };
    // The use case and logical views are laid out by ortho itself rather than ELK,
    // since their shape is fully determined.
    let (graph, laid_out) = match kind {
        DiagramKind::UseCase => {
            let graph = extract_use_case_graph(ast, exposed);
            let laid_out = layout_use_cases(&graph);
            (graph, laid_out)
        }
        DiagramKind::Logical => {
            let graph = extract_logical_graph(ast, exposed, &workspace.packages());
            let laid_out = layout_tree(&graph);
            (graph, laid_out)
        }
        DiagramKind::Process => {
            let mut sequence = extract_sequence_model(ast, exposed);
            for lifeline in &mut sequence.lifelines {
                lifeline.source = source_mark(workspace, lifeline.element);
            }
            return Ok(RenderResult {
                svg: render_sequence_svg(&sequence, title, theme),
                summary: format!("{} lifeline(s), {} message(s)", sequence.lifelines.len(), sequence.messages.len()),
            });
        }
        DiagramKind::Implementation => {
            let options = LayoutOptions { grid_leaves: true, ..LayoutOptions::default() };
            elk(extract_implementation_graph(ast, exposed), options)?
        }
        DiagramKind::Physical => elk(extract_physical_graph(ast, exposed), LayoutOptions::default())?,
        DiagramKind::Deployment => {
            let options = LayoutOptions { algorithm: Algorithm::RectPacking, ..LayoutOptions::default() };
            elk(extract_deployment_graph(ast, exposed, &workspace.packages()), options)?
        }
    };
    let mut laid_out = laid_out;
    for node in &mut laid_out.nodes {
        node.source = source_mark(workspace, node.element);
    }
    Ok(RenderResult {
        svg: render_svg(&laid_out, title, theme),
        summary: format!("{} node(s), {} edge(s)", laid_out.nodes.len(), graph.edges.len()),
    })
}

/// Where an element a box stands for is declared, the file named from the working
/// folder as the diagram's title names it.
fn source_mark(workspace: &Workspace, element: Option<NodeId>) -> Option<SourceMark> {
    let (file, line) = workspace.declared_at(element?)?;
    Some(SourceMark { file: relative_to_working_folder(file), line })
}

/// Renders a view as the diagram its kind calls for, of what it exposes.
pub fn render_view(
    workspace: &Workspace,
    view: NodeId,
    title: &DiagramTitle,
    theme: &Theme,
) -> Result<RenderResult, String> {
    let ast = &workspace.ast;
    render_exposed(workspace, diagram_kind(ast, view), &exposed_elements(ast, view), title, theme)
}

/// A view rendered to the file named after it.
pub struct RenderedView {
    pub view: NodeId,
    /// The file it is written to, named after the view.
    pub file: String,
    pub result: RenderResult,
    /// Why it is unreadable on landscape A4, when it is.
    pub warning: Option<String>,
}

/// A view ortho does not draw, and why.
pub struct SkippedView {
    pub view: NodeId,
    pub reason: String,
}

pub struct ModelRendering {
    pub rendered: Vec<RenderedView>,
    pub skipped: Vec<SkippedView>,
}

/// Renders every view a model declares, in the order of their kinds, each to a file named after it, headed by its
/// name and naming the file that declares it, relative to the working folder. A view
/// ortho does not draw is passed over with the reason; two views that would be written
/// to the same file are an error.
pub fn render_views(workspace: &Workspace, theme: &Theme) -> Result<ModelRendering, String> {
    let ast = &workspace.ast;
    let mut skipped = Vec::new();
    let mut by_file: Vec<(String, NodeId, &str)> = Vec::new();
    for document in &workspace.documents {
        for view in declared_views(ast, document.root) {
            let file = view_file_name(ast, view);
            if let Some(reason) = why_undrawable(ast, view) {
                skipped.push(SkippedView { view, reason });
            } else if let Some((_, other, _)) = by_file.iter().find(|(taken, _, _)| *taken == file) {
                return Err(format!(
                    "'{}' and '{}' would both be written to {file}; give one a short name, view <name> '...'.",
                    view_heading(ast, *other),
                    view_heading(ast, view)
                ));
            } else {
                by_file.push((file, view, &document.path));
            }
        }
    }
    // Written, and so listed and shown, use case view first and process view last,
    // whatever the files are called; views of one kind keep their files' order.
    by_file.sort_by_key(|(_, view, _)| diagram_kind(ast, *view));
    let mut rendered = Vec::with_capacity(by_file.len());
    for (file, view, path) in by_file {
        let title = DiagramTitle {
            heading: ast.text(view, Prop::Name).unwrap_or("").to_string(),
            definition: ast.text(view, Prop::Definition).map(String::from),
            source: relative_to_working_folder(path),
        };
        let result = render_view(workspace, view, &title, theme)?;
        let (width, height) = svg_size(&result.svg);
        let warning = page_warning(view_heading(ast, view), width, height);
        rendered.push(RenderedView { view, file, result, warning });
    }
    Ok(ModelRendering { rendered, skipped })
}

/// A path as seen from the working folder, with forward slashes on every system, as
/// Node's `path.relative` gives it: `examples/coffee-machine/logical.sysml`, or
/// `../other/model.sysml` for one outside it.
pub fn relative_to_working_folder(path: &str) -> String {
    let (Ok(path), Ok(working)) = (std::path::absolute(path), std::env::current_dir()) else {
        return path.replace('\\', "/");
    };
    let (path, working) = (normal(&path), normal(&working));
    // A file on another drive has no relative path; Node gives it whole.
    if path.first() != working.first() {
        return path.iter().collect::<PathBuf>().to_string_lossy().replace('\\', "/");
    }
    let shared = path.iter().zip(&working).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".to_string(); working.len() - shared];
    parts.extend(path[shared..].iter().map(|part| part.as_os_str().to_string_lossy().into_owned()));
    parts.join("/")
}

/// A path's parts without `.`, and with each `..` taking the part before it away.
fn normal(path: &Path) -> Vec<Component<'_>> {
    let mut parts: Vec<Component> = Vec::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir if matches!(parts.last(), Some(Component::Normal(_))) => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts
}
