//! Which views a model declares, and how ortho draws each.
use crate::parser::ast::{Ast, NodeId, Prop, Type};
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// The standard view definitions ortho draws (§9.2.20).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StandardView {
    GeneralView,
    InterconnectionView,
    SequenceView,
}

impl StandardView {
    pub fn name(self) -> &'static str {
        match self {
            StandardView::GeneralView => "GeneralView",
            StandardView::InterconnectionView => "InterconnectionView",
            StandardView::SequenceView => "SequenceView",
        }
    }

    /// Whether a view of this definition may expose `element`.
    pub fn may_expose(self, ast: &Ast, element: NodeId) -> bool {
        let ty = ast[element].ty;
        match self {
            StandardView::GeneralView => {
                matches!(ty, Type::PackageDecl | Type::ActionDef | Type::UseCaseDef | Type::UseCaseUsage)
            }
            StandardView::InterconnectionView => ty == Type::PartUsage,
            // A scenario: an `occurrence def`, or an `action def`, which is one too.
            StandardView::SequenceView => {
                ty == Type::ActionDef
                    || (ty == Type::OtherDefinition && ast.text(element, Prop::Kind) == Some("occurrence"))
            }
        }
    }

    /// What a view of this definition may expose, as its error says it.
    fn exposes(self) -> &'static str {
        match self {
            StandardView::GeneralView => "packages, action definitions or use cases",
            StandardView::InterconnectionView => "parts",
            StandardView::SequenceView => "an occurrence definition",
        }
    }
}

/// Which of ortho's six diagrams a view is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiagramKind {
    UseCase,
    Logical,
    Implementation,
    Physical,
    Deployment,
    Process,
}

impl DiagramKind {
    /// As the recordings and the TypeScript name it.
    pub fn name(self) -> &'static str {
        match self {
            DiagramKind::UseCase => "use-case",
            DiagramKind::Logical => "logical",
            DiagramKind::Implementation => "implementation",
            DiagramKind::Physical => "physical",
            DiagramKind::Deployment => "deployment",
            DiagramKind::Process => "process",
        }
    }
}

/// The standard view a view is defined by, matched by its last name, since the
/// standard library is not loaded.
pub fn standard_view(ast: &Ast, view: NodeId) -> Option<StandardView> {
    standard_view_named(ast.text(view, Prop::Definition)?)
}

fn standard_view_named(definition: &str) -> Option<StandardView> {
    match definition.rsplit("::").next()? {
        "GeneralView" => Some(StandardView::GeneralView),
        "InterconnectionView" => Some(StandardView::InterconnectionView),
        "SequenceView" => Some(StandardView::SequenceView),
        _ => None,
    }
}

/// What a view exposes and is found.
pub fn exposed_elements(ast: &Ast, view: NodeId) -> Vec<NodeId> {
    ast.children(view, Prop::Exposes).filter_map(|expose| ast.target(expose, Prop::Element)).collect()
}

/// Why ortho cannot draw a view, or nothing when it can. A model may declare any
/// view; one other than these is skipped, for this reason.
pub fn why_undrawable(ast: &Ast, view: NodeId) -> Option<String> {
    let Some(definition) = standard_view(ast, view) else {
        let written = ast.text(view, Prop::Definition).unwrap_or("");
        return Some(format!("ortho draws a GeneralView, an InterconnectionView or a SequenceView, not '{written}'."));
    };
    let article = if definition == StandardView::InterconnectionView { "An" } else { "A" };
    exposed_elements(ast, view)
        .into_iter()
        .any(|element| !definition.may_expose(ast, element))
        .then(|| format!("{article} {} exposes {}.", definition.name(), definition.exposes()))
}

/// The views a file declares.
pub fn declared_views(ast: &Ast, root: NodeId) -> Vec<NodeId> {
    ast.with_all_contents(root).into_iter().skip(1).filter(|node| ast[*node].ty == Type::ViewUsage).collect()
}

/// A diagram's heading is its view's name (§7.26.5), or for a view without one its definition's.
pub fn view_heading(ast: &Ast, view: NodeId) -> &str {
    ast.text(view, Prop::Name).or_else(|| ast.text(view, Prop::Definition)).unwrap_or("")
}

/// The file a view is rendered to: its short name, or else its name, in lower case
/// with hyphens. `view 'Logical view — Coffee Machine'` is written to
/// logical-view-coffee-machine.svg, `view <logical> 'Logical view'` to logical.svg.
pub fn view_file_name(ast: &Ast, view: NodeId) -> String {
    let named = ast.text(view, Prop::ShortName).unwrap_or_else(|| view_heading(ast, view));
    let lower: String = named.nfkd().filter(|c| !is_combining_mark(*c)).collect::<String>().to_lowercase();
    let mut slug = String::new();
    for c in lower.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    format!("{}.svg", if slug.is_empty() { "view" } else { slug })
}

/// Which of ortho's six diagrams a view of `definition` exposing `exposed` is. An
/// InterconnectionView is the physical view and a SequenceView the process view. A
/// GeneralView presents whatever it exposes, so its content decides: a function
/// tree, with any use cases it refines, use cases, allocations, or else packages of
/// software.
pub fn diagram_kind_of(ast: &Ast, definition: Option<StandardView>, exposed: &[NodeId]) -> DiagramKind {
    match definition {
        Some(StandardView::InterconnectionView) => return DiagramKind::Physical,
        Some(StandardView::SequenceView) => return DiagramKind::Process,
        _ => {}
    }
    if exposed.iter().any(|element| ast[*element].ty == Type::ActionDef) {
        return DiagramKind::Logical;
    }
    let contents: Vec<Type> =
        exposed.iter().flat_map(|element| ast.with_all_contents(*element)).map(|node| ast[node].ty).collect();
    if contents.iter().any(|ty| matches!(ty, Type::UseCaseDef | Type::UseCaseUsage)) {
        DiagramKind::UseCase
    } else if contents.contains(&Type::AllocationUsage) {
        DiagramKind::Deployment
    } else {
        DiagramKind::Implementation
    }
}

/// Which of ortho's six diagrams a view is.
pub fn diagram_kind(ast: &Ast, view: NodeId) -> DiagramKind {
    diagram_kind_of(ast, standard_view(ast, view), &exposed_elements(ast, view))
}
