//! The use case view: what it reads of a model, and how it stands it on the page.
use ortho::diagrams::use_case::extract_use_case_graph;
use ortho::layout::use_case::layout_use_cases;
use ortho::layout::{LaidOutDiagram, LaidOutNode};
use ortho::model::graph::{DiagramGraph, EdgeKind, NodeShape};
use ortho::parser::ast::{Prop, Type};
use ortho::parser::workspace::Workspace;
use ortho::render::svg::render_svg;
use ortho::render::theme::LIGHT;

mod common;
use common::{centre, close_to, everything, example, fixture, names, parse_ok, sorted_names, title};

fn graph_of(workspace: &Workspace) -> DiagramGraph {
    extract_use_case_graph(&workspace.ast, &everything(workspace))
}

fn node<'a>(laid_out: &'a LaidOutDiagram, name: &str) -> &'a LaidOutNode {
    laid_out.nodes.iter().find(|n| n.name == name).unwrap_or_else(|| panic!("no {name}"))
}

#[test]
fn parses_use_case_defs_and_usages_with_actors_subjects_and_includes() {
    let workspace = parse_ok(
        "package P {
            part def Operator;
            part def System;
            use case def DoThing {
                subject sys : System;
                actor op : Operator;
            }
            use case doThing : DoThing {
                include use case helpTask;
                include otherTask;
            }
            use case otherTask;
            use case helpTask;
            part def Station {
                perform doThing;
            }
        }",
    );
    let ast = &workspace.ast;
    let package = workspace.packages()[0];
    let members: Vec<_> = ast.children(package, Prop::Members).collect();
    let definition = *members.iter().find(|m| ast[**m].ty == Type::UseCaseDef).unwrap();
    assert!(ast.children(definition, Prop::Members).any(|m| ast[m].ty == Type::ActorUsage));
    let usage = *members
        .iter()
        .find(|m| ast[**m].ty == Type::UseCaseUsage && ast.text(**m, Prop::Name) == Some("doThing"))
        .unwrap();
    let typed = ast.target(usage, Prop::Type).unwrap();
    assert_eq!(ast.text(typed, Prop::Name), Some("DoThing"));
    // `include use case x` declares an included use case; `include x` names one.
    let includes: Vec<_> = ast.children(usage, Prop::Members).filter(|m| ast[*m].ty == Type::IncludeUsage).collect();
    let [declared, named] = includes[..] else { panic!("{includes:?}") };
    assert!(ast.flag(declared, Prop::UseCase));
    assert_eq!(ast.text(declared, Prop::Name), Some("helpTask"));
    let end = ast.child(named, Prop::Target).unwrap();
    let last = *ast.references_of(end, Prop::Segments).last().unwrap();
    assert_eq!(ast.text(ast.reference_target(last).unwrap(), Prop::Name), Some("otherTask"));
}

#[test]
fn builds_use_cases_one_subject_deduplicated_actors_and_edges() {
    let graph = graph_of(&fixture("use-case.sysml"));

    // Each use case is a «use case» box, named as declared; the FlyMission def is
    // represented by its usage, not shown itself.
    let use_cases: Vec<_> = graph.nodes.iter().filter(|n| n.keyword == "use case").collect();
    assert!(use_cases.iter().all(|n| n.shape == NodeShape::Usage));
    assert_eq!(
        sorted_names(use_cases.iter().copied()),
        ["avoidObstacles", "capturePhotos", "flyMission : FlyMission", "returnToHome"]
    );
    assert!(!graph.nodes.iter().any(|n| n.name == "FlyMission"));

    // Four use cases declare the same subject: one «subject» box, joined to each.
    let subjects: Vec<_> = graph.nodes.iter().filter(|n| n.keyword == "subject").collect();
    assert_eq!(names(subjects.iter().copied()), ["drone : Drone"]);
    let mut joined: Vec<&str> =
        graph.edges.iter().filter(|e| e.target_id == subjects[0].id).map(|e| e.source_id.as_str()).collect();
    joined.sort();
    let mut use_case_ids: Vec<&str> = use_cases.iter().map(|n| n.id.as_str()).collect();
    use_case_ids.sort();
    assert_eq!(joined, use_case_ids);

    let actors: Vec<_> = graph.nodes.iter().filter(|n| n.shape == NodeShape::Actor).collect();
    assert_eq!(names(actors.iter().copied()), ["pilot : Pilot"]);

    // The pilot's lines: flyMission (inherited), returnToHome, capturePhotos.
    let mut lines: Vec<&str> = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Association && e.source_id == actors[0].id)
        .map(|e| e.target_id.as_str())
        .collect();
    lines.sort();
    assert_eq!(lines, ["DroneUseCases::capturePhotos", "DroneUseCases::flyMission", "DroneUseCases::returnToHome"]);

    let mut includes: Vec<&str> =
        graph.edges.iter().filter(|e| e.kind == EdgeKind::Include).map(|e| e.target_id.as_str()).collect();
    includes.sort();
    assert_eq!(includes, ["DroneUseCases::avoidObstacles", "DroneUseCases::returnToHome"]);
}

#[test]
fn draws_a_use_case_an_include_declares_as_a_box_of_its_own() {
    let graph = graph_of(&parse_ok(
        "package P {
            part def System;
            use case def Help { subject system : System; }
            use case main { include use case help : Help; }
        }",
    ));
    assert_eq!(names(graph.nodes.iter().filter(|n| n.keyword == "use case")), ["main", "help : Help"]);
    // As for any usage, the def it is typed by is not shown itself, and what the
    // def declares comes with it.
    assert!(!graph.nodes.iter().any(|n| n.name == "Help"));
    assert_eq!(names(graph.nodes.iter().filter(|n| n.keyword == "subject")), ["system : System"]);

    let laid_out = layout_use_cases(&graph);
    let includes: Vec<(&str, &str)> = laid_out
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Include)
        .map(|e| (e.source_id.as_str(), e.target_id.as_str()))
        .collect();
    assert_eq!(includes, [("P::main", "P::main::help")]);
}

#[test]
fn keeps_a_use_case_an_include_declares_apart_from_one_of_the_same_name() {
    let graph = graph_of(&parse_ok(
        "package P {
            use case help;
            use case a { include use case help; }
            use case b { include help; }
        }",
    ));
    let includes: Vec<(&str, &str)> = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Include)
        .map(|e| (e.source_id.as_str(), e.target_id.as_str()))
        .collect();
    assert_eq!(includes, [("P::a", "P::a::help"), ("P::b", "P::help")]);
}

#[test]
fn stands_the_subject_above_the_use_cases_and_the_actor_below_them_and_renders_them() {
    let laid_out = layout_use_cases(&graph_of(&fixture("use-case.sysml")));

    // As OMG draws a use case (§7.25): its subject above it.
    let actor = laid_out.nodes.iter().find(|n| n.shape == NodeShape::Actor).unwrap();
    let subject = laid_out.nodes.iter().find(|n| n.keyword == "subject").unwrap();
    let use_cases: Vec<_> = laid_out.nodes.iter().filter(|n| n.keyword == "use case").collect();
    assert_eq!(use_cases.len(), 4);
    for use_case in use_cases {
        assert!(subject.y + subject.height < use_case.y);
        assert!(use_case.y + use_case.height < actor.y);
    }

    let svg = render_svg(&laid_out, &title("Scenarios — DroneUseCases", "examples/phase2-usecases.sysml"), &LIGHT);
    assert!(svg.contains("&#171;use case&#187;"));
    assert!(svg.contains("&#171;subject&#187;"));
    assert!(svg.contains("<circle")); // the actor's head
    // An «include» is a solid arrow (§8.2.3.25).
    assert!(svg.contains("&#171;include&#187;"));
    assert!(svg.contains(r#"marker-end="url(#openArrow)""#));
    assert!(!svg.contains("stroke-dasharray"));
}

#[test]
fn stands_a_primary_actor_before_a_secondary_one_each_under_its_use_cases() {
    // The customer comes with every order, so it is declared on the def: primary.
    // The courier joins one use case only: secondary.
    let laid_out = layout_use_cases(&graph_of(&parse_ok(
        "package Shop {
            part def Customer;
            part def Courier;
            use case def Order {
                subject Webshop;
                actor customer : Customer;
            }
            use case browse : Order;
            use case checkout : Order;
            use case deliver : Order {
                subject Webshop;
                actor courier : Courier;
            }
        }",
    )));
    let middle = |name: &str| {
        let n = node(&laid_out, name);
        centre(n.x, n.width)
    };
    let customer = node(&laid_out, "customer : Customer");
    let courier = node(&laid_out, "courier : Courier");
    assert_eq!(laid_out.nodes.iter().filter(|n| n.shape == NodeShape::Actor).count(), 2);

    // The customer centred under its three use cases, the courier after it, under deliver.
    let orders = ["browse : Order", "checkout : Order", "deliver : Order"];
    let mean = orders.iter().map(|name| middle(name)).sum::<f64>() / 3.0;
    assert!(close_to(middle("customer : Customer"), mean, 2));
    assert!(courier.x > customer.x + customer.width);
    assert!(close_to(middle("courier : Courier"), middle("deliver : Order"), 2));

    // Each actor is joined to its own use cases and no others; the customer's come
    // through the def, deliver's included.
    let joined = |actor: &str| {
        let mut ends: Vec<&str> = laid_out
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Association && (e.source_id == actor || e.target_id == actor))
            .map(|e| if e.source_id == actor { e.target_id.as_str() } else { e.source_id.as_str() })
            .collect();
        ends.sort();
        ends
    };
    assert_eq!(joined(&customer.id), ["Shop::browse", "Shop::checkout", "Shop::deliver"]);
    assert_eq!(joined(&courier.id), ["Shop::deliver"]);
}

#[test]
fn stands_each_use_case_between_the_use_cases_it_includes() {
    let row = |name: &str| {
        let laid_out = layout_use_cases(&graph_of(&example(name, &["use-case.sysml"])));
        let mut use_cases: Vec<_> = laid_out.nodes.iter().filter(|n| n.keyword == "use case").collect();
        use_cases.sort_by(|a, b| a.x.total_cmp(&b.x));
        use_cases.iter().map(|n| n.name.split(" : ").next().unwrap().to_string()).collect::<Vec<_>>()
    };
    // Surveying includes planning and following: one on either side.
    assert_eq!(row("survey-drone"), ["planSurvey", "surveyArea", "followProgress"]);
    assert_eq!(row("coffee-machine"), ["makeCappuccino", "makeEspresso", "descale"]);
}

#[test]
fn draws_a_subject_a_use_case_restates_in_the_box_of_its_defs_subject() {
    // A use case adding an actor states its subject first; restated, the subject
    // keeps its def's type.
    let graph = graph_of(&parse_ok(
        "package Shop {
            part def Webshop;
            part def Customer;
            part def Courier;
            use case def Order {
                subject shop : Webshop;
                actor customer : Customer;
            }
            use case browse : Order;
            use case deliver : Order {
                subject shop;
                actor courier : Courier;
            }
        }",
    ));
    let subjects: Vec<_> = graph.nodes.iter().filter(|n| n.keyword == "subject").collect();
    assert_eq!(names(subjects.iter().copied()), ["shop : Webshop"]);
    assert_eq!(graph.edges.iter().filter(|e| e.target_id == subjects[0].id).count(), 2);
}

#[test]
fn draws_every_line_straight_from_outline_to_outline_clear_of_every_box_it_does_not_join() {
    let inside = |x: f64, y: f64, n: &LaidOutNode, by: f64| {
        x > n.x - by && x < n.x + n.width + by && y > n.y - by && y < n.y + n.height + by
    };
    for name in ["survey-drone", "coffee-machine"] {
        let laid_out = layout_use_cases(&graph_of(&example(name, &["use-case.sysml"])));
        let boxes: Vec<_> =
            laid_out.nodes.iter().filter(|n| n.shape == NodeShape::Usage || n.shape == NodeShape::Definition).collect();
        for edge in &laid_out.edges {
            let line = format!("{name}: {} -> {}", edge.source_id, edge.target_id);
            let [p, q] = edge.points[..] else { panic!("{line} is not straight") };
            // An end on a box lies on its outline: within its bounds, and no deeper
            // than a rounded corner's arc.
            for (end, id) in [(p, &edge.source_id), (q, &edge.target_id)] {
                if let Some(b) = boxes.iter().find(|n| n.id == *id) {
                    assert!(
                        inside(end.x, end.y, b, 0.5) && !inside(end.x, end.y, b, -3.5),
                        "{line} ends off {}",
                        b.name
                    );
                }
            }
            for n in boxes.iter().filter(|b| b.id != edge.source_id && b.id != edge.target_id) {
                for i in 0..=200 {
                    let x = p.x + (q.x - p.x) * i as f64 / 200.0;
                    let y = p.y + (q.y - p.y) * i as f64 / 200.0;
                    assert!(!inside(x, y, n, 0.0), "{line} crosses {}", n.name);
                }
            }
        }
    }
}
