//! A view, from what it exposes to its SVG, or for a table of requirements its Markdown.
use crate::diagrams::deployment::extract_deployment_graph;
use crate::diagrams::implementation::extract_implementation_graph;
use crate::diagrams::logical::extract_logical_graph;
use crate::diagrams::physical::extract_physical_graph;
use crate::diagrams::requirements::{documentation, extract_requirements, has_derivation, has_verification_case};
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
use crate::render::requirements::{Specification, render_requirements_markdown, requirements_summary};
use crate::render::sequence::render_sequence_svg;
use crate::render::svg::{DiagramTitle, render_svg};
use crate::render::theme::Theme;
use std::path::{Component, Path, PathBuf};

pub struct RenderResult {
    /// The SVG, or for a table of requirements the Markdown.
    pub content: String,
    pub summary: String,
}

/// Renders the diagram a view of `kind` exposing `exposed` shows, or why ELK could not lay it out.
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
        DiagramKind::Requirements => return Err("A table of requirements is no diagram.".to_string()),
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
                content: render_sequence_svg(&sequence, title, theme),
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
        content: render_svg(&laid_out, title, theme),
        summary: format!("{} node(s), {} edge(s)", laid_out.nodes.len(), graph.edges.len()),
    })
}

/// Where an element a box stands for is declared, the file named from the working
/// folder as the diagram's title names it.
fn source_mark(workspace: &Workspace, element: Option<NodeId>) -> Option<SourceMark> {
    let (file, line) = workspace.declared_at(element?)?;
    Some(SourceMark { file: relative_to_working_folder(file), line })
}

/// Renders a view as the diagram its kind calls for, of what it exposes, or as a
/// table of requirements whose links lead from the folder `out` it is written to.
pub fn render_view(
    workspace: &Workspace,
    view: NodeId,
    title: &DiagramTitle,
    theme: &Theme,
    out: &str,
) -> Result<RenderResult, String> {
    let ast = &workspace.ast;
    match diagram_kind(ast, view) {
        DiagramKind::Requirements => Ok(render_requirements(workspace, view, out)),
        kind => render_exposed(workspace, kind, &exposed_elements(ast, view), title, theme),
    }
}

/// A GridView of requirements as Markdown, written to the folder `out`, each
/// element that satisfies a requirement linked to its line in the model.
fn render_requirements(workspace: &Workspace, view: NodeId, out: &str) -> RenderResult {
    let ast = &workspace.ast;
    let roots: Vec<NodeId> = workspace.documents.iter().map(|document| document.root).collect();
    let rows = extract_requirements(ast, &roots, &exposed_elements(ast, view));
    let link = |element: NodeId| {
        let (file, line) = workspace.declared_at(element)?;
        Some(format!("{}#L{line}", link_path(file, out)))
    };
    let file = workspace.declared_at(view).map_or("", |(file, _)| file);
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    let specification = Specification {
        heading: view_heading(ast, view),
        source: (name, link_path(file, out)),
        introduction: documentation(ast, view),
        verification: has_verification_case(ast, &roots),
        derivation: has_derivation(ast, &roots),
    };
    RenderResult {
        content: render_requirements_markdown(&specification, &rows, &link),
        summary: requirements_summary(&rows, specification.verification),
    }
}

/// A file as a Markdown link in the folder `out` leads to it, a space or a bracket
/// in its name escaped: `../my%20model.sysml`.
fn link_path(file: &str, out: &str) -> String {
    relative_path(file, out).replace(' ', "%20").replace('(', "%28").replace(')', "%29")
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
/// to the same file are an error. A table of requirements links to the model from
/// the folder `out` it is written to.
pub fn render_views(workspace: &Workspace, theme: &Theme, out: &str) -> Result<ModelRendering, String> {
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
        let result = render_view(workspace, view, &title, theme, out)?;
        let warning = (diagram_kind(ast, view) != DiagramKind::Requirements)
            .then(|| {
                let (width, height) = svg_size(&result.content);
                page_warning(view_heading(ast, view), width, height)
            })
            .flatten();
        rendered.push(RenderedView { view, file, result, warning });
    }
    Ok(ModelRendering { rendered, skipped })
}

/// A path as seen from the working folder, with forward slashes on every system, as
/// Node's `path.relative` gives it: `examples/coffee-machine/logical.sysml`, or
/// `../other/model.sysml` for one outside it.
pub fn relative_to_working_folder(path: &str) -> String {
    relative_path(path, ".")
}

/// A path as seen from the folder `from`, both relative to the working folder unless
/// absolute, with forward slashes on every system. A path that leaves the folder only
/// because one of the two is reached through a link, as macOS's `/var` is
/// `/private/var`, is compared by the folders' real paths instead.
pub fn relative_path(path: &str, from: &str) -> String {
    let (Ok(path), Ok(working)) = (std::path::absolute(path), std::path::absolute(from)) else {
        return path.replace('\\', "/");
    };
    let named = relative_parts(&path, &working);
    if !named.starts_with("..") {
        return named;
    }
    let real = relative_parts(&real_path(&path), &real_path(&working));
    let climbs = |relative: &str| relative.split('/').take_while(|part| *part == "..").count();
    // Kept as named when the real paths are on different drives, given whole.
    if !Path::new(&real).is_absolute() && climbs(&real) < climbs(&named) { real } else { named }
}

/// A path with the links in its folders followed, as far as they exist; the file
/// itself, and the parts not made yet, kept as named.
fn real_path(path: &Path) -> PathBuf {
    let path: PathBuf = normal(path).iter().collect();
    let mut folder = path.parent();
    while let Some(existing) = folder {
        if let Ok(real) = existing.canonicalize() {
            return real.join(path.strip_prefix(existing).unwrap_or(&path));
        }
        folder = existing.parent();
    }
    path
}

/// `path` as seen from the folder `working`, both absolute.
fn relative_parts(path: &Path, working: &Path) -> String {
    let (path, working) = (normal(path), normal(working));
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
