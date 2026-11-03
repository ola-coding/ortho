//! Which views a model declares and what each draws.
use ortho::diagrams::logical::extract_logical_graph;
use ortho::diagrams::sequence::extract_sequence_model;
use ortho::diagrams::use_case::extract_use_case_graph;
use ortho::layout::tree::layout_tree;
use ortho::layout::use_case::layout_use_cases;
use ortho::model::graph::EdgeKind;
use ortho::model::views::{
    DiagramKind, declared_views, diagram_kind, exposed_elements, view_file_name, view_heading, why_undrawable,
};
use ortho::parser::workspace::parse_sysml_files;
use ortho::pipeline::{relative_to_working_folder, render_view, render_views};
use ortho::render::svg::DiagramTitle;
use ortho::render::theme::LIGHT;
use std::path::Path;

mod common;
use common::{parse_ok, views};

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

#[test]
fn renders_the_examples_as_their_committed_diagrams() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // And the views written by hand, for what no example draws.
    for (example, count) in [("examples/coffee-machine", 6), ("examples/survey-drone", 6), ("tests/views", 8)] {
        let folder = root.join(example);
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
                let result = render_view(&workspace, view, &title, &LIGHT).unwrap();
                let committed = folder.join("diagrams").join(view_file_name(ast, view));
                // As Git on Windows may check it out.
                let expected = std::fs::read_to_string(&committed).unwrap().replace("\r\n", "\n");
                assert!(result.svg == expected, "{} is not what {example} renders to", committed.display());
                rendered += 1;
            }
        }
        assert_eq!(rendered, count, "{example}: every view");
    }
}

#[test]
fn renders_every_view_it_can_draw_passes_over_the_rest_and_refuses_two_views_for_one_file() {
    let workspace = parse_ok(
        "package Every {
            action def A { action b; }
            view one : GeneralView { expose A; }
            view states : StateTransitionView { expose A; }
            view two : GeneralView { expose A; }
        }",
    );
    let rendering = render_views(&workspace, &LIGHT).unwrap();
    assert_eq!(rendering.rendered.iter().map(|r| r.file.as_str()).collect::<Vec<_>>(), ["one.svg", "two.svg"]);
    let skipped: Vec<(&str, &str)> =
        rendering.skipped.iter().map(|s| (view_heading(&workspace.ast, s.view), s.reason.as_str())).collect();
    assert_eq!(
        skipped,
        [("states", "ortho draws a GeneralView, an InterconnectionView or a SequenceView, not 'StateTransitionView'.")]
    );
    let twice = parse_ok(
        "package Twice {
            action def A;
            view 'Functions' : GeneralView { expose A; }
            view functions : GeneralView { expose A; }
        }",
    );
    let error = render_views(&twice, &LIGHT).err().unwrap();
    assert!(error.starts_with("'Functions' and 'functions' would both be written to functions.svg"), "{error}");
}

#[test]
fn names_the_file_that_declares_a_view_as_seen_from_the_working_folder() {
    let working = std::env::current_dir().unwrap();
    let inside = working.join("examples").join("model.sysml");
    assert_eq!(relative_to_working_folder(&inside.to_string_lossy()), "examples/model.sysml");
    assert_eq!(relative_to_working_folder("./examples/../model.sysml"), "model.sysml");
    let beside = working.parent().unwrap().join("other").join("model.sysml");
    assert_eq!(relative_to_working_folder(&beside.to_string_lossy()), "../other/model.sysml");
}
