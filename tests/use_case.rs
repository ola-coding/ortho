//! The use case view: what it reads of a model, and how it stands it on the page.
use ortho::diagrams::use_case::extract_use_case_graph;
use ortho::layout::use_case::layout_use_cases;
use ortho::layout::{LaidOutDiagram, LaidOutNode};
use ortho::model::graph::{DiagramGraph, EdgeKind, NodeShape};
use ortho::parser::ast::{Prop, Type};
use ortho::parser::workspace::Workspace;
use ortho::render::page::PAGE_WIDTH;
use ortho::render::svg::render_svg;
use ortho::render::theme::LIGHT;

mod common;
use common::{centre, close_to, everything, example, fixture, names, parse_ok, sorted_names, title};

fn graph_of(workspace: &Workspace) -> DiagramGraph {
    extract_use_case_graph(&workspace.ast, &everything(workspace))
}

fn node_by_id<'a>(laid_out: &'a LaidOutDiagram, id: &str) -> &'a LaidOutNode {
    laid_out.nodes.iter().find(|n| n.id == id).unwrap_or_else(|| panic!("no {id}"))
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
fn tells_actors_of_one_type_apart_by_their_names() {
    // Using the machine and looking after it are both a user's, but in roles of
    // their own: each figure is named as its use case def declares it.
    let graph = graph_of(&parse_ok(
        "package P {
            part def User;
            part def CoffeeMachine;
            use case def OperateMachine {
                subject machine : CoffeeMachine;
                actor user : User;
            }
            use case def ServiceMachine {
                subject machine : CoffeeMachine;
                actor serviceGuy : User;
            }
            use case makeEspresso : OperateMachine;
            use case descale : ServiceMachine;
        }",
    ));
    let actors_of = |use_case: &str| -> Vec<&str> {
        let lines = graph.edges.iter().filter(|e| e.kind == EdgeKind::Association && e.target_id == use_case);
        let actors = lines.filter_map(|e| graph.nodes.iter().find(|n| n.id == e.source_id));
        actors.filter(|n| n.shape == NodeShape::Actor).map(|n| n.name.as_str()).collect()
    };
    assert_eq!(actors_of("P::makeEspresso"), ["user : User"]);
    assert_eq!(actors_of("P::descale"), ["serviceGuy : User"]);
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
fn draws_each_use_case_with_its_subject_above_it_and_its_actor_to_its_left_and_renders_them() {
    let laid_out = layout_use_cases(&graph_of(&fixture("use-case.sysml")));

    // As SysML v2 draws a use case (Part 1, Table 23): its subject above it, its
    // actors beside it. Each use case no other includes has its own of both: they
    // are its parameters.
    let use_cases: Vec<_> = laid_out.nodes.iter().filter(|n| n.keyword == "use case").collect();
    assert_eq!(use_cases.len(), 4);
    let joined = |use_case: &LaidOutNode, other: &dyn Fn(&LaidOutNode) -> bool| -> Vec<&LaidOutNode> {
        laid_out
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Association && (e.source_id == use_case.id || e.target_id == use_case.id))
            .flat_map(|e| laid_out.nodes.iter().filter(|n| n.id == e.source_id || n.id == e.target_id))
            .filter(|n| other(n))
            .collect()
    };
    let mut groups = 0;
    for use_case in use_cases {
        let subjects = joined(use_case, &|n| n.keyword == "subject");
        let actors = joined(use_case, &|n| n.shape == NodeShape::Actor);
        if let [subject] = subjects[..] {
            groups += 1;
            assert!(subject.y + subject.height < use_case.y);
            assert!(close_to(centre(subject.x, subject.width), centre(use_case.x, use_case.width), 2));
            let [actor] = actors[..] else { panic!("{} has no actor of its own", use_case.name) };
            assert!(actor.x + actor.width < use_case.x);
        }
    }
    assert!(groups >= 2, "{groups}");

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
fn stands_a_primary_actor_to_the_left_of_its_use_case_and_a_secondary_one_to_the_right() {
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
    // One customer beside each use case, each joined to its own only.
    let customers: Vec<_> = laid_out.nodes.iter().filter(|n| n.name == "customer : Customer").collect();
    assert_eq!(customers.len(), 3);
    for customer in customers {
        let lines: Vec<_> =
            laid_out.edges.iter().filter(|e| e.kind == EdgeKind::Association && e.source_id == customer.id).collect();
        let [line] = lines[..] else { panic!("{} has {} lines", customer.id, lines.len()) };
        let use_case = node_by_id(&laid_out, &line.target_id);
        assert!(customer.x + customer.width < use_case.x);
        // Level with the use case, the line straight across from the actor's hand.
        assert_eq!(line.points[0].y, line.points[1].y);
        assert_eq!(line.points[1].x, use_case.x);
    }
    let deliver = node(&laid_out, "deliver : Order");
    let courier = node(&laid_out, "courier : Courier");
    assert!(courier.x > deliver.x + deliver.width);
    let to_courier: Vec<_> = laid_out.edges.iter().filter(|e| e.target_id == courier.id).collect();
    assert_eq!(to_courier.len(), 1);
    assert_eq!(to_courier[0].source_id, deliver.id);
}

#[test]
fn draws_the_use_cases_a_use_case_includes_below_it_in_the_order_it_includes_them() {
    let at = |example_name: &str| {
        let laid_out = layout_use_cases(&graph_of(&example(example_name, &["use-case.sysml"])));
        move |name: &str| {
            let n = laid_out.nodes.iter().find(|n| n.name.split(" : ").next() == Some(name)).unwrap();
            (centre(n.x, n.width), n.y, n.height)
        }
    };
    // Surveying includes planning and following: both below it, side by side.
    let survey = at("survey-drone");
    let (area_x, area_y, area_height) = survey("surveyArea");
    let (plan_x, plan_y, _) = survey("planSurvey");
    let (follow_x, follow_y, _) = survey("followProgress");
    assert!(plan_y > area_y + area_height);
    assert_eq!(plan_y, follow_y);
    assert!(plan_x < area_x && area_x < follow_x);

    // A cappuccino includes an espresso, straight below it; descaling stands beside.
    let coffee = at("coffee-machine");
    let (cappuccino_x, cappuccino_y, cappuccino_height) = coffee("makeCappuccino");
    let (espresso_x, espresso_y, _) = coffee("makeEspresso");
    let (descale_x, descale_y, _) = coffee("descale");
    assert!(close_to(espresso_x, cappuccino_x, 2));
    assert!(espresso_y > cappuccino_y + cappuccino_height);
    assert_eq!(descale_y, cappuccino_y);
    assert!(descale_x > cappuccino_x);
}

#[test]
fn sets_out_the_use_cases_in_rows_as_wide_as_landscape_a4_in_the_order_they_are_declared() {
    let use_cases: String = (0..12).map(|i| format!("use case task{i} : Task;\n")).collect();
    let laid_out = layout_use_cases(&graph_of(&parse_ok(&format!(
        "package P {{
            part def System;
            part def User;
            use case def Task {{ subject system : System; actor user : User; }}
            {use_cases}
        }}"
    ))));
    assert!(laid_out.width <= PAGE_WIDTH, "{}", laid_out.width);
    let at: Vec<(f64, f64)> = (0..12)
        .map(|i| {
            let n = node(&laid_out, &format!("task{i} : Task"));
            (n.x, n.y)
        })
        .collect();
    let rows = at.windows(2).filter(|pair| pair[1].1 > pair[0].1).count() + 1;
    assert!(rows > 1, "{rows}");
    // Left to right, then the next row down.
    for pair in at.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        assert!(y1 > y0 || (y1 == y0 && x1 > x0), "{pair:?}");
    }
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
fn draws_every_line_at_right_angles_from_outline_to_outline_clear_of_every_box_it_does_not_join() {
    let inside = |x: f64, y: f64, n: &LaidOutNode, by: f64| {
        x > n.x - by && x < n.x + n.width + by && y > n.y - by && y < n.y + n.height + by
    };
    for name in ["survey-drone", "coffee-machine"] {
        let laid_out = layout_use_cases(&graph_of(&example(name, &["use-case.sysml"])));
        let boxes: Vec<_> =
            laid_out.nodes.iter().filter(|n| n.shape == NodeShape::Usage || n.shape == NodeShape::Definition).collect();
        for edge in &laid_out.edges {
            let line = format!("{name}: {} -> {}", edge.source_id, edge.target_id);
            let (p, q) = (edge.points[0], edge.points[edge.points.len() - 1]);
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
            for leg in edge.points.windows(2) {
                let (a, b) = (leg[0], leg[1]);
                assert!(a.x == b.x || a.y == b.y, "{line} runs aslant");
                for n in boxes.iter().filter(|n| n.id != edge.source_id && n.id != edge.target_id) {
                    for i in 0..=200 {
                        let x = a.x + (b.x - a.x) * i as f64 / 200.0;
                        let y = a.y + (b.y - a.y) * i as f64 / 200.0;
                        assert!(!inside(x, y, n, 0.0), "{line} crosses {}", n.name);
                    }
                }
            }
        }
    }
}
