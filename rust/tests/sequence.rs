//! The process view: scenarios, their messages, and the lifelines they pass between.
use ortho::diagrams::sequence::{SequenceModel, extract_sequence_model};
use ortho::parser::ast::{Ast, NodeId, Prop, Type};
use ortho::parser::workspace::Workspace;
use ortho::render::sequence::render_sequence_svg;
use ortho::render::theme::LIGHT;

mod common;
use common::{count, everything, fixture, parse_ok, title};

fn model_of(workspace: &Workspace) -> SequenceModel {
    extract_sequence_model(&workspace.ast, &everything(workspace))
}

fn members_of(ast: &Ast, owner: NodeId, ty: Type) -> Vec<NodeId> {
    ast.children(owner, Prop::Members).filter(|m| ast[*m].ty == ty).collect()
}

/// The name of what the first segment of a message's end refers to.
fn first_segment(ast: &Ast, message: NodeId, end: Prop) -> Option<&str> {
    let end = ast.child(message, end)?;
    let segment = *ast.references_of(end, Prop::Segments).first()?;
    ast.text(ast.reference_target(segment)?, Prop::Name)
}

fn labels(model: &SequenceModel) -> Vec<&str> {
    model.messages.iter().map(|m| m.label.as_str()).collect()
}

fn lifelines(model: &SequenceModel) -> Vec<&str> {
    model.lifelines.iter().map(|l| l.label.as_str()).collect()
}

#[test]
fn parses_an_occurrence_def_with_ordered_messages() {
    let workspace = fixture("process.sysml");
    let ast = &workspace.ast;
    let scenario = members_of(ast, workspace.packages()[0], Type::OtherDefinition)
        .into_iter()
        .find(|d| ast.text(*d, Prop::Kind) == Some("occurrence"))
        .unwrap();
    let messages = members_of(ast, scenario, Type::MessageUsage);
    assert_eq!(messages.len(), 6);
    assert!(!ast.flag(messages[0], Prop::Then));
    assert!(ast.flag(messages[1], Prop::Then));
    assert_eq!(ast.text(messages[0], Prop::Name), Some("cmdTakeOff"));
    assert_eq!(ast.text(ast.target(messages[0], Prop::Payload).unwrap(), Prop::Name), Some("TakeOffCommand"));
    assert_eq!(first_segment(ast, messages[0], Prop::Source), Some("pilot"));
    assert_eq!(first_segment(ast, messages[0], Prop::Target), Some("fc"));
}

#[test]
fn parses_nested_actions_and_lifeline_parts_local_to_an_action_def() {
    let workspace = parse_ok(
        "package P {
            action def Handshake {
                part client;
                part server;
                message syn from client to server;
                then message ack from server to client;
                action retry;
            }
        }",
    );
    let ast = &workspace.ast;
    let action = members_of(ast, workspace.packages()[0], Type::ActionDef)[0];
    let messages = members_of(ast, action, Type::MessageUsage);
    assert_eq!(first_segment(ast, messages[0], Prop::Source), Some("client"));
}

#[test]
fn orders_lifelines_by_first_appearance_and_keeps_message_order() {
    let model = model_of(&fixture("process.sysml"));
    assert_eq!(
        lifelines(&model),
        ["pilot : Pilot", "fc : FlightControllerSw", "nav : NavigationSw", "motors : MotorControl"]
    );
    assert_eq!(
        labels(&model),
        ["cmdTakeOff of TakeOffCommand", "planRoute", "computePath", "waypoints", "motorSetpoints", "telemetry"]
    );
    let to_itself = &model.messages[2];
    assert_eq!(to_itself.source_id, to_itself.target_id);
}

#[test]
fn renders_lifelines_messages_and_a_self_loop_to_svg() {
    let model = model_of(&fixture("process.sysml"));
    let svg =
        render_sequence_svg(&model, &title("Process View — DroneProcess", "examples/phase4-sequence.sysml"), &LIGHT);
    assert!(svg.contains(r#"stroke-dasharray="4 4""#)); // lifelines
    assert!(svg.contains(r#"marker-end="url(#msgArrow)""#));
    assert!(svg.contains("cmdTakeOff of TakeOffCommand"));
    assert!(svg.contains("<polyline")); // the self-message's loop
    // A rounded «part» head on every lifeline and no activation bars (§8.2.3.9):
    // the only other rectangles are the page and the frame.
    let heads = model.lifelines.len();
    assert_eq!(count(&svg, "&#171;part&#187;"), heads);
    assert_eq!(count(&svg, "<rect "), heads + 2);
    // Messages appear top to bottom in the order declared.
    assert!(svg.find("planRoute").unwrap() < svg.find("telemetry").unwrap());
}

#[test]
fn labels_a_message_as_declared_name_of_payload_either_alone_or_message() {
    let model = model_of(&parse_ok(
        "package P {
            attribute def Order;
            action def Shop {
                part client;
                part server;
                message buy of Order from client to server;
                then message of Order from client to server;
                then message ack from server to client;
                then message from server to client;
            }
        }",
    ));
    assert_eq!(labels(&model), ["buy of Order", "Order", "ack", "«message»"]);
}

#[test]
fn heads_every_lifeline_with_a_part_box_a_person_too() {
    let model = model_of(&parse_ok(
        "package P {
            part def Server;
            action def Handshake {
                part client;
                part server : Server;
                message syn from client to server;
            }
        }",
    ));
    assert_eq!(lifelines(&model), ["client", "server : Server"]);
    let svg = render_sequence_svg(&model, &title("Process view — P", "inline"), &LIGHT);
    // A stick figure is for a use case's actor (§8.2.3.21); a lifeline's head is a
    // part (§8.2.3.9).
    assert_eq!(count(&svg, "&#171;part&#187;"), 2);
    assert!(!svg.contains("<circle"));
}
