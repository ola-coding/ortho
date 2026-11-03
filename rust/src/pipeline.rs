//! A view, from what it exposes to its SVG.
use crate::diagrams::logical::extract_logical_graph;
use crate::diagrams::sequence::extract_sequence_model;
use crate::diagrams::use_case::extract_use_case_graph;
use crate::layout::tree::layout_tree;
use crate::layout::use_case::layout_use_cases;
use crate::model::views::{DiagramKind, diagram_kind, exposed_elements};
use crate::parser::ast::NodeId;
use crate::parser::workspace::Workspace;
use crate::render::sequence::render_sequence_svg;
use crate::render::svg::{DiagramTitle, render_svg};
use crate::render::theme::Theme;

pub struct RenderResult {
    pub svg: String,
    pub summary: String,
}

/// Renders what a view of `kind` exposing `exposed` shows, or nothing for a kind
/// ELK lays out, which is not ported yet.
pub fn render_exposed(
    workspace: &Workspace,
    kind: DiagramKind,
    exposed: &[NodeId],
    title: &DiagramTitle,
    theme: &Theme,
) -> Option<RenderResult> {
    let ast = &workspace.ast;
    // Laid out by ortho itself rather than ELK, where the view's shape is fully determined.
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
            let sequence = extract_sequence_model(ast, exposed);
            return Some(RenderResult {
                svg: render_sequence_svg(&sequence, title, theme),
                summary: format!("{} lifeline(s), {} message(s)", sequence.lifelines.len(), sequence.messages.len()),
            });
        }
        DiagramKind::Implementation | DiagramKind::Physical | DiagramKind::Deployment => return None,
    };
    Some(RenderResult {
        svg: render_svg(&laid_out, title, theme),
        summary: format!("{} node(s), {} edge(s)", laid_out.nodes.len(), graph.edges.len()),
    })
}

/// Renders a view as the diagram its kind calls for, of what it exposes.
pub fn render_view(workspace: &Workspace, view: NodeId, title: &DiagramTitle, theme: &Theme) -> Option<RenderResult> {
    let ast = &workspace.ast;
    render_exposed(workspace, diagram_kind(ast, view), &exposed_elements(ast, view), title, theme)
}
