//! Which views a model declares and what each draws. `oracle.rs` compares every stage
//! of every view with what the TypeScript produces.
use ortho::diagrams::logical::extract_logical_graph;
use ortho::diagrams::sequence::extract_sequence_model;
use ortho::diagrams::use_case::extract_use_case_graph;
use ortho::layout::tree::layout_tree;
use ortho::layout::use_case::layout_use_cases;
use ortho::model::graph::EdgeKind;
use ortho::model::views::{
    DiagramKind, declared_views, diagram_kind, exposed_elements, view_file_name, view_heading, why_undrawable,
};
use ortho::parser::ast::NodeId;
use ortho::parser::workspace::{Severity, Workspace, parse_sysml_files};
use ortho::pipeline::render_view;
use ortho::render::svg::DiagramTitle;
use ortho::render::theme::LIGHT;
use std::path::Path;

fn parse_ok(text: &str) -> Workspace {
    let workspace = Workspace::build(vec![("model.sysml".to_string(), text.to_string())])
        .unwrap_or_else(|failures| panic!("{failures:?}"));
    assert_eq!(workspace.reported(Severity::Error), Vec::<String>::new());
    workspace
}

fn views(workspace: &Workspace) -> Vec<NodeId> {
    declared_views(&workspace.ast, workspace.documents[0].root)
}

// A function tree and a scenario side by side: each nests actions, and each view
// draws only what it exposes.
const TWO_VIEWS: &str = "
    package M {
        part def Engine;
        action def Operate {
            action start;
            action stop;
        }
        action def Scenario {
            part a : Engine;
            part b : Engine;
            message ping from a to b;
        }
        view 'Functions — M' : GeneralView { expose Operate; }
        view sequence : SequenceView { expose Scenario; }
    }
";

#[test]
fn heads_a_diagram_with_its_view_name_without_the_quotes() {
    let workspace = parse_ok(TWO_VIEWS);
    let ast = &workspace.ast;
    let [functions, sequence] = views(&workspace)[..] else { panic!() };
    assert_eq!(view_heading(ast, functions), "Functions — M");
    assert_eq!(view_heading(ast, sequence), "sequence");
    assert_eq!(
        [diagram_kind(ast, functions), diagram_kind(ast, sequence)],
        [DiagramKind::Logical, DiagramKind::Process]
    );
}

#[test]
fn draws_only_what_a_view_exposes() {
    let workspace = parse_ok(TWO_VIEWS);
    let ast = &workspace.ast;
    let [functions, sequence] = views(&workspace)[..] else { panic!() };
    // The scenario nests actions too, but the function view does not expose it.
    let graph = extract_logical_graph(ast, &exposed_elements(ast, functions), &workspace.packages());
    assert_eq!(graph.nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(), ["Operate", "start", "stop"]);
    let model = extract_sequence_model(ast, &exposed_elements(ast, sequence));
    assert_eq!(model.messages.iter().map(|m| m.label.as_str()).collect::<Vec<_>>(), ["ping"]);
}

#[test]
fn draws_a_sequence_view_of_an_occurrence_def_as_omg_writes_a_scenario() {
    let workspace = parse_ok(
        "package M {
            private import StandardViewDefinitions::*;
            part def Engine;
            item def Go;
            occurrence def Scenario {
                part a : Engine;
                part b : Engine;
                message ping of Go from a to b;
            }
            view sequence : SequenceView { expose Scenario; }
        }",
    );
    let ast = &workspace.ast;
    let view = views(&workspace)[0];
    assert_eq!(why_undrawable(ast, view), None);
    assert_eq!(diagram_kind(ast, view), DiagramKind::Process);
    let model = extract_sequence_model(ast, &exposed_elements(ast, view));
    assert_eq!(model.messages.iter().map(|m| m.label.as_str()).collect::<Vec<_>>(), ["ping of Go"]);
}

#[test]
fn loads_any_view_and_says_why_it_cannot_draw_one_other_than_the_three_it_knows() {
    let why = |text: &str| {
        let workspace = parse_ok(text);
        views(&workspace).into_iter().map(|view| why_undrawable(&workspace.ast, view)).collect::<Vec<_>>()
    };
    assert_eq!(why(TWO_VIEWS), [None, None]);
    assert_eq!(
        why("package M { action def A; view v : StateTransitionView { expose A; } }"),
        [Some(
            "ortho draws a GeneralView, an InterconnectionView or a SequenceView, not 'StateTransitionView'.".into()
        )]
    );
    assert_eq!(
        why("package M { action def A; view v : InterconnectionView { expose A; } }"),
        [Some("An InterconnectionView exposes parts.".into())]
    );
    assert_eq!(
        why("package M { part p; view v : GeneralView { expose p; } }"),
        [Some("A GeneralView exposes packages, action definitions or use cases.".into())]
    );
}

#[test]
fn takes_a_view_that_exposes_only_a_use_case_for_the_use_case_view() {
    let workspace = parse_ok("package M { use case def Order; view v : GeneralView { expose Order; } }");
    let view = views(&workspace)[0];
    assert_eq!(why_undrawable(&workspace.ast, view), None);
    assert_eq!(diagram_kind(&workspace.ast, view), DiagramKind::UseCase);
}

#[test]
fn writes_each_view_to_a_file_named_after_its_short_name_or_else_its_name() {
    let workspace = parse_ok(
        "package Files {
            action def A;
            view 'Logical view — Coffee Machine' : GeneralView { expose A; }
            view <logical> 'Logical view' : GeneralView { expose A; }
            view 'Översikt: Kaffe & Te' : GeneralView { expose A; }
            view '— ' : GeneralView { expose A; }
        }",
    );
    let names: Vec<String> = views(&workspace).into_iter().map(|view| view_file_name(&workspace.ast, view)).collect();
    assert_eq!(names, ["logical-view-coffee-machine.svg", "logical.svg", "oversikt-kaffe-te.svg", "view.svg"]);
}

#[test]
fn joins_actors_and_the_subject_to_their_use_cases_and_points_an_include_at_what_it_includes() {
    let workspace = parse_ok(
        "package M {
            part def Machine;
            part def User;
            use case def Order {
                subject machine : Machine;
                actor user : User;
            }
            use case order : Order {
                include pay;
            }
            use case pay : Order;
            view v : GeneralView { expose M; }
        }",
    );
    let ast = &workspace.ast;
    let graph = extract_use_case_graph(ast, &exposed_elements(ast, views(&workspace)[0]));
    let names: Vec<&str> = graph.nodes.iter().map(|n| n.name.as_str()).collect();
    // The def is drawn through its usages, the subject and the actor once for both.
    assert_eq!(names, ["order : Order", "pay : Order", "machine : Machine", "user : User"]);
    let edges: Vec<(EdgeKind, &str, &str)> =
        graph.edges.iter().map(|e| (e.kind, e.source_id.as_str(), e.target_id.as_str())).collect();
    assert_eq!(
        edges,
        [
            (EdgeKind::Association, "M::order", "subject:machine : Machine"),
            (EdgeKind::Association, "actor:M::User", "M::order"),
            (EdgeKind::Association, "M::pay", "subject:machine : Machine"),
            (EdgeKind::Association, "actor:M::User", "M::pay"),
            (EdgeKind::Include, "M::order", "M::pay"),
        ]
    );
    let laid_out = layout_use_cases(&graph);
    let at = |id: &str| laid_out.nodes.iter().find(|n| n.id == id).unwrap();
    // The subject above the row of use cases, the actor below it.
    assert!(at("subject:machine : Machine").y < at("M::order").y);
    assert!(at("actor:M::User").y > at("M::order").y);
    assert_eq!(at("M::order").y, at("M::pay").y);
}

#[test]
fn lists_leaves_beneath_their_parent_and_spreads_any_other_family_in_a_row() {
    let workspace = parse_ok(
        "package M {
            action def Root {
                action a { action a1; action a2; }
                action b { action b1; }
            }
            view v : GeneralView { expose Root; }
        }",
    );
    let ast = &workspace.ast;
    let graph = extract_logical_graph(ast, &exposed_elements(ast, views(&workspace)[0]), &workspace.packages());
    let laid_out = layout_tree(&graph);
    let at = |name: &str| laid_out.nodes.iter().find(|n| n.name == name).unwrap();
    assert_eq!(at("a").y, at("b").y);
    assert!(at("a").x < at("b").x);
    assert_eq!(at("a1").x, at("a2").x);
    assert!(at("a1").y < at("a2").y);
    assert!(at("Root").y < at("a").y);
}

/// The examples' views ortho lays out itself render as their committed diagrams.
#[test]
fn renders_the_examples_as_their_committed_diagrams() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for example in ["coffee-machine", "survey-drone"] {
        let folder = root.join("examples").join(example);
        let mut files: Vec<String> = std::fs::read_dir(&folder)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "sysml"))
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        files.sort();
        let workspace = parse_sysml_files(&files).unwrap().workspace;
        let ast = &workspace.ast;
        let mut rendered = 0;
        for document in &workspace.documents {
            for view in declared_views(ast, document.root) {
                let source = Path::new(&document.path).strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                let title = DiagramTitle {
                    heading: view_heading(ast, view).to_string(),
                    definition: ast.text(view, ortho::parser::ast::Prop::Definition).map(String::from),
                    source,
                };
                let Some(result) = render_view(&workspace, view, &title, &LIGHT) else {
                    continue;
                };
                let committed = folder.join("diagrams").join(view_file_name(ast, view));
                // As Git on Windows may check it out.
                let expected = std::fs::read_to_string(&committed).unwrap().replace("\r\n", "\n");
                assert!(result.svg == expected, "{} is not what {example} renders to", committed.display());
                rendered += 1;
            }
        }
        assert_eq!(rendered, 3, "{example}: the use case, logical and process views");
    }
}
