//! Names, imports and what a model reports. `oracle.rs` compares every reference with
//! what Langium found.
use ortho::parser::ast::{NodeId, Prop, Type};
use ortho::parser::workspace::{Severity, Workspace, parse_sysml_files};
use std::path::Path;

fn build(text: &str) -> Workspace {
    Workspace::build(vec![("model.sysml".to_string(), text.to_string())])
        .unwrap_or_else(|failures| panic!("{failures:?}"))
}

fn reported(workspace: &Workspace, severity: Severity) -> Vec<String> {
    let of_severity = workspace.documents.iter().flat_map(|document| &document.diagnostics);
    of_severity.filter(|d| d.severity == severity).map(|d| d.message.clone()).collect()
}

fn errors_of(text: &str) -> Vec<String> {
    reported(&build(text), Severity::Error)
}

fn parse_ok(text: &str) -> Workspace {
    let workspace = build(text);
    assert_eq!(reported(&workspace, Severity::Error), Vec::<String>::new());
    workspace
}

fn unresolved(kind: &str, name: &str) -> String {
    format!("Could not resolve reference to {kind} named '{name}'.")
}

fn package(workspace: &Workspace, index: usize) -> NodeId {
    workspace.packages()[index]
}

fn member(workspace: &Workspace, owner: NodeId, name: &str) -> NodeId {
    let ast = &workspace.ast;
    ast.children(owner, Prop::Members).find(|m| ast.text(*m, Prop::Name) == Some(name)).unwrap()
}

fn first_of(workspace: &Workspace, owner: NodeId, ty: Type) -> NodeId {
    let ast = &workspace.ast;
    ast.children(owner, Prop::Members).find(|m| ast[*m].ty == ty).unwrap()
}

/// What each segment of a connector end names.
fn segment_names(workspace: &Workspace, end: NodeId) -> Vec<Option<&str>> {
    let ast = &workspace.ast;
    let segments = ast.references_of(end, Prop::Segments).into_iter();
    segments.map(|segment| ast.reference_target(segment).and_then(|target| ast.text(target, Prop::Name))).collect()
}

fn end_of(workspace: &Workspace, usage: NodeId, prop: Prop) -> NodeId {
    workspace.ast.child(usage, prop).unwrap()
}

const LIBRARY: &str = "
    package ImpSoftware {
        package Control { part def BrewController; part def MilkController; }
        part sw { part video; part other; }
    }
    package ImpHardware {
        part hw { part cpu; }
    }
";

#[test]
fn resolves_a_usage_to_its_definition() {
    let workspace = parse_ok(
        "package P {
            part def Battery { part cell : Cell [4]; }
            part def Cell;
        }",
    );
    let ast = &workspace.ast;
    let battery = member(&workspace, package(&workspace, 0), "Battery");
    let cell = member(&workspace, battery, "cell");
    assert_eq!(ast.text(ast.target(cell, Prop::Type).unwrap(), Prop::Name), Some("Cell"));
}

#[test]
fn resolves_specialization_and_cross_package_qualified_names() {
    let workspace = parse_ok(
        "package A { part def Base; }
         package B { part def Derived :> A::Base; }",
    );
    let ast = &workspace.ast;
    let derived = member(&workspace, package(&workspace, 1), "Derived");
    let supertype = ast.references_of(derived, Prop::Supertypes)[0];
    assert_eq!(ast.text(ast.reference_target(supertype).unwrap(), Prop::Name), Some("Base"));
}

#[test]
fn resolves_connect_feature_chains_through_part_types() {
    let workspace = parse_ok(
        "package P {
            port def Pw;
            part def Battery { port outlet : Pw; }
            part def Fc { port in_ : Pw; }
            part def Drone {
                part battery : Battery;
                part fc : Fc;
                connect battery.outlet to fc.in_;
            }
        }",
    );
    let drone = member(&workspace, package(&workspace, 0), "Drone");
    let connection = first_of(&workspace, drone, Type::ConnectionUsage);
    assert_eq!(
        segment_names(&workspace, end_of(&workspace, connection, Prop::Source)),
        [Some("battery"), Some("outlet")]
    );
    assert_eq!(segment_names(&workspace, end_of(&workspace, connection, Prop::Target)), [Some("fc"), Some("in_")]);
}

#[test]
fn resolves_connector_chains_ending_in_an_attribute() {
    let workspace = parse_ok(
        "package P {
            part def Sensor { attribute rate : Real; }
            part def Logger { attribute input : Real; }
            part def Board {
                part sensor : Sensor;
                part logger : Logger;
                connect sensor.rate to logger.input;
            }
        }",
    );
    let board = member(&workspace, package(&workspace, 0), "Board");
    let connection = first_of(&workspace, board, Type::ConnectionUsage);
    assert_eq!(segment_names(&workspace, end_of(&workspace, connection, Prop::Source))[1], Some("rate"));
    assert_eq!(segment_names(&workspace, end_of(&workspace, connection, Prop::Target))[1], Some("input"));
}

#[test]
fn parses_interface_defs_and_imports() {
    parse_ok(
        "package Ifaces {
            port def Pw;
            interface def PowerIface {
                end supply : Pw;
                end demand : Pw;
            }
        }
        package User {
            private import Ifaces::*;
            part def X;
        }",
    );
}

#[test]
fn reports_unresolved_references_as_errors() {
    assert_eq!(errors_of("package P { part def A :> Missing; }"), [unresolved("Definition", "Missing")]);
}

// A realization connects its internals to the ports it inherits from the block it
// specializes, so those names must resolve without redeclaring them.
#[test]
fn resolves_connector_ends_against_ports_inherited_through_specialization() {
    let workspace = parse_ok(
        "package P {
            port def DataPort;
            part def Block { port ctrlIn : DataPort; }
            part def Motor { port ctrlIn : DataPort; }
            part def Design :> Block {
                part motor : Motor;
                connect ctrlIn to motor.ctrlIn;
            }
        }",
    );
    let ast = &workspace.ast;
    let design = member(&workspace, package(&workspace, 0), "Design");
    let connection = first_of(&workspace, design, Type::ConnectionUsage);
    let source = end_of(&workspace, connection, Prop::Source);
    let first = ast.reference_target(ast.references_of(source, Prop::Segments)[0]).unwrap();
    assert_eq!(ast.text(first, Prop::Name), Some("ctrlIn"));
    assert_eq!(ast.text(ast[first].parent.unwrap(), Prop::Name), Some("Block"));
}

#[test]
fn a_locally_declared_feature_shadows_an_inherited_one_of_the_same_name() {
    let workspace = parse_ok(
        "package P {
            port def DataPort;
            part def Block { port ctrlIn : DataPort; }
            part def Motor { port ctrlIn : DataPort; }
            part def Design :> Block {
                port ctrlIn : DataPort;
                part motor : Motor;
                connect ctrlIn to motor.ctrlIn;
            }
        }",
    );
    let ast = &workspace.ast;
    let design = member(&workspace, package(&workspace, 0), "Design");
    let connection = first_of(&workspace, design, Type::ConnectionUsage);
    let source = end_of(&workspace, connection, Prop::Source);
    let first = ast.reference_target(ast.references_of(source, Prop::Segments)[0]).unwrap();
    assert_eq!(ast.text(ast[first].parent.unwrap(), Prop::Name), Some("Design"));
}

#[test]
fn makes_a_packages_members_visible_by_their_own_names() {
    let workspace = parse_ok(&format!(
        "{LIBRARY}
        package ImpDeployment {{
            private import ImpSoftware::*;
            private import ImpHardware::*;
            private import ScalarValues::*;
            allocate sw.video to hw.cpu;
            part brew : Control::BrewController;
        }}"
    ));
    let allocation = first_of(&workspace, package(&workspace, 2), Type::AllocationUsage);
    assert_eq!(segment_names(&workspace, end_of(&workspace, allocation, Prop::Source)), [Some("sw"), Some("video")]);
    assert_eq!(segment_names(&workspace, end_of(&workspace, allocation, Prop::Target)), [Some("hw"), Some("cpu")]);
}

#[test]
fn imports_one_element_a_packages_own_members_outrank_imports_which_reach_nested_packages() {
    assert_eq!(
        errors_of(&format!(
            "{LIBRARY}
            package ImpOne {{
                private import ImpSoftware::sw;
                allocate sw.video to ImpHardware::hw.cpu;
            }}"
        )),
        Vec::<String>::new()
    );
    assert_eq!(
        errors_of(&format!(
            "{LIBRARY}
            package ImpOnly {{
                private import ImpSoftware::Control::BrewController;
                part def P {{ part b : BrewController; part m : MilkController; }}
            }}"
        )),
        [unresolved("Definition", "MilkController")]
    );
    let workspace = parse_ok(&format!(
        "{LIBRARY}
        package ImpOwn {{
            private import ImpSoftware::*;
            part sw {{ part video; }}
            package Inner {{
                allocate sw.video to ImpHardware::hw.cpu;
                part def Brew {{ part brew : Control::BrewController; }}
            }}
        }}"
    ));
    let ast = &workspace.ast;
    let own = package(&workspace, 2);
    let inner = member(&workspace, own, "Inner");
    let allocation = first_of(&workspace, inner, Type::AllocationUsage);
    let source = end_of(&workspace, allocation, Prop::Source);
    let first = ast.reference_target(ast.references_of(source, Prop::Segments)[0]).unwrap();
    assert_eq!(ast[first].parent, Some(own));
}

#[test]
fn passes_on_public_imports_only() {
    let units = "
        package ImpParts { part def Engine; part def Motor; }
        package ImpUnits {
            private import ImpParts::Engine;
            public import ImpParts::Motor;
        }
    ";
    assert_eq!(
        errors_of(&format!(
            "{units}
            package ImpUser {{
                private import ImpUnits::*;
                part def M {{ part m : Motor; }}
                part def Q {{ part q : ImpUnits::Motor; }}
            }}"
        )),
        Vec::<String>::new()
    );
    assert_eq!(
        errors_of(&format!(
            "{units}
            package ImpPrivate {{
                private import ImpUnits::*;
                part def E {{ part e : Engine; }}
            }}"
        )),
        [unresolved("Definition", "Engine")]
    );
}

#[test]
fn resolves_nothing_through_a_package_that_is_not_imported() {
    assert_eq!(
        errors_of(&format!(
            "{LIBRARY}
            package ImpNone {{
                allocate sw.video to ImpHardware::hw.cpu;
            }}"
        )),
        [unresolved("Feature", "sw")]
    );
}

#[test]
fn reads_a_quoted_name_as_the_name_without_its_quotes_in_declarations_and_references() {
    let workspace = parse_ok(
        "package 'Name Hardware' {
            part def 'Brew unit' { port 'water in' : 'Water port'; }
            port def 'Water port';
            part def Machine {
                part brew : 'Brew unit';
                part tank : 'Name Hardware'::Tank;
                connect brew.'water in' to tank.outlet;
            }
            part def Tank { port outlet : 'Water port'; }
        }",
    );
    let ast = &workspace.ast;
    let machine = member(&workspace, package(&workspace, 0), "Machine");
    let type_name =
        |part: &str| ast.text(ast.target(member(&workspace, machine, part), Prop::Type).unwrap(), Prop::Name);
    assert_eq!((type_name("brew"), type_name("tank")), (Some("Brew unit"), Some("Tank")));
    let connection = first_of(&workspace, machine, Type::ConnectionUsage);
    assert_eq!(
        segment_names(&workspace, end_of(&workspace, connection, Prop::Source)),
        [Some("brew"), Some("water in")]
    );
}

#[test]
fn finds_an_element_by_its_short_name_or_an_alias() {
    let workspace = parse_ok(
        "package <NS> 'Name Short' {
            part def <BU> 'Brew unit';
            alias Unit for BU;
            part a : BU;
            part b : Unit;
            part c : NS::BU;
        }",
    );
    let ast = &workspace.ast;
    let pkg = package(&workspace, 0);
    for part in ["a", "b", "c"] {
        let target = ast.target(member(&workspace, pkg, part), Prop::Type).unwrap();
        assert_eq!(ast.text(target, Prop::Name), Some("Brew unit"), "{part}");
    }
}

#[test]
fn imports_everything_nested_in_a_package_with_a_double_star() {
    parse_ok(
        "package NameDeep { package Inner { part def Pump; } }
         package NameUser {
             private import all NameDeep::**;
             part pump : Pump;
         }",
    );
}

#[test]
fn takes_a_message_from_and_to_event_occurrences_on_the_parts_that_have_them() {
    let workspace = parse_ok(
        "package KindEvents {
            part def Driver { event occurrence turnOn; }
            part def Car { event occurrence started; }
            action def Start {
                part driver : Driver;
                part car : Car;
                message go from driver.turnOn to car.started;
            }
        }",
    );
    let ast = &workspace.ast;
    let start = member(&workspace, package(&workspace, 0), "Start");
    let message = first_of(&workspace, start, Type::MessageUsage);
    let source = end_of(&workspace, message, Prop::Source);
    let types: Vec<Type> =
        ast.references_of(source, Prop::Segments).iter().map(|s| ast[ast.reference_target(*s).unwrap()].ty).collect();
    assert_eq!(types, [Type::PartUsage, Type::OtherUsage]);
}

#[test]
fn names_an_unnamed_redefinition_and_perform_after_what_they_refer_to() {
    let workspace = parse_ok(
        "package KindNames {
            action def Work { action step; }
            action work : Work;
            part def Base { part arm; }
            part def Robot :> Base {
                part :>> arm;
                perform work.step;
            }
            part robot : Robot;
            allocate robot.arm to robot.step;
        }",
    );
    let ast = &workspace.ast;
    let allocation = first_of(&workspace, package(&workspace, 0), Type::AllocationUsage);
    let target = end_of(&workspace, allocation, Prop::Target);
    let types: Vec<Type> =
        ast.references_of(target, Prop::Segments).iter().map(|s| ast[ast.reference_target(*s).unwrap()].ty).collect();
    assert_eq!(types, [Type::PartUsage, Type::PerformUsage]);
}

#[test]
fn lets_a_view_name_a_library_element_without_loading_the_library_and_still_catches_a_typo() {
    // SpatialItem is declared in SpatialItems; MassValue in ISQBase, which ISQ imports publicly.
    assert_eq!(
        errors_of(
            "package LibraryNames {
                private import SpatialItems::*;
                private import ISQ::*;
                part def Car :> SpatialItem;
                part car : SpatialItems::SpatialItem;
                action def Weigh {
                    part scale : Car;
                    part car : Car;
                    message weight of MassValue from scale to car;
                }
            }"
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        errors_of("package LibraryTypo { private import SpatialItems::*; part car : SpatialIten; }"),
        [unresolved("Definition", "SpatialIten")]
    );
}

#[test]
fn rejects_a_reserved_word_as_a_name_unless_it_is_quoted() {
    for word in ["frame", "until"] {
        let unquoted = format!("package P {{ part def Drone {{ part {word}; }} }}");
        assert!(Workspace::build(vec![("model.sysml".to_string(), unquoted)]).is_err());
        assert_eq!(errors_of(&format!("package P {{ part def Drone {{ part '{word}'; }} }}")), Vec::<String>::new());
    }
}

#[test]
fn rejects_a_qualified_name_into_a_definition_and_accepts_the_chain_from_a_usage() {
    let model = |target: &str| {
        format!(
            "package H {{ part def M {{ part c; }} part m : M; }}
             package S {{ part sw {{ part a; }} }}
             package D {{ allocate S::sw.a to {target}; }}"
        )
    };
    assert_eq!(
        errors_of(&model("H::M::c")),
        ["'H::M::c' names a feature through its owner. Start at a usage and continue with dots."]
    );
    assert_eq!(errors_of(&model("H::m.c")), Vec::<String>::new());
}

#[test]
fn requires_a_use_case_to_declare_its_subject_before_its_actors() {
    let model = |body: &str| {
        format!(
            "package U {{
                part def Operator;
                use case def Operate {{ subject Machine; actor operator : Operator; }}
                use case service : Operate {{ {body} }}
            }}"
        )
    };
    assert_eq!(
        errors_of(&model("actor technician : Operator;")),
        ["Declare the subject of 'service' before its actors: the subject is a use case's first parameter."]
    );
    assert_eq!(errors_of(&model("subject Machine; actor technician : Operator;")), Vec::<String>::new());
}

#[test]
fn performs_an_action_through_a_chain_that_starts_at_a_usage() {
    let model = |target: &str| {
        format!(
            "package Functions {{
                action def Survey {{ action communicate {{ action streamVideo; }} }}
                action survey : Survey;
            }}
            package Software {{
                part def Streamer {{ port data; perform {target}; }}
            }}"
        )
    };
    assert_eq!(errors_of(&model("Functions::survey.communicate.streamVideo")), Vec::<String>::new());
    // The spec's own rule, and the pilot implementation's: a path into a
    // definition's features is not a feature chain.
    assert_eq!(
        errors_of(&model("Functions::Survey::communicate::streamVideo")),
        [
            "'Functions::Survey::communicate::streamVideo' names a feature through its owner. Start at a usage and continue with dots."
        ]
    );
}

#[test]
fn refuses_to_perform_something_that_is_not_an_action_or_a_use_case() {
    assert_eq!(
        errors_of("package P { part def Streamer { port data; perform data; } }"),
        ["'data' is not an action or a use case, so it cannot be performed."]
    );
}

#[test]
fn gives_a_port_directed_features_and_conjugates_the_port_that_needs_them() {
    parse_ok(
        "package P {
            port def Api {
                in attribute command : String;
                out attribute status : String;
                inout attribute parameter : Real;
            }
            interface def Link {
                end offered : Api;
                end needed : ~Api;
            }
            part def Server { port api : Api; }
            part def Client { port api : ~Api; }
        }",
    );
}

#[test]
fn resolves_a_message_payload_to_an_attribute_def() {
    let text = |payload: &str| {
        format!(
            "package P {{
                attribute def Command;
                part def Sender; part def Receiver;
                action def Talk {{
                    part s : Sender; part r : Receiver;
                    message go of {payload} from s to r;
                }}
            }}"
        )
    };
    assert_eq!(errors_of(&text("Command")), Vec::<String>::new());
    assert!(!errors_of(&text("Undeclared")).is_empty());
}

const LIBRARY_WARNING: &str = "Package 'UseCases' has the name of a standard library package and hides it, and other tools may find the library's instead. Rename it.";

#[test]
fn warns_of_a_package_named_after_a_standard_library_package_as_the_model_still_loads() {
    let workspace = build("package UseCases { use case def A; }");
    assert_eq!(reported(&workspace, Severity::Warning), [LIBRARY_WARNING]);
    assert_eq!(reported(&workspace, Severity::Error), Vec::<String>::new());
    // Quoted, it is the same name.
    assert_eq!(reported(&build("package 'UseCases';"), Severity::Warning), [LIBRARY_WARNING]);
}

#[test]
fn leaves_alone_any_other_package_and_the_library_itself() {
    for text in [
        "package SystemUseCases;",
        "package Outer { package UseCases; }",
        "standard library package UseCases;",
        "library package Views;",
        "package toString;",
    ] {
        assert_eq!(reported(&build(text), Severity::Warning), Vec::<String>::new(), "{text}");
    }
}

#[test]
fn hands_a_warning_back_with_its_file_and_line_and_does_not_stop_the_model_loading() {
    let folder = std::env::temp_dir().join(format!("ortho-linking-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("model.sysml");
    std::fs::write(&file, "// A note.\npackage UseCases { use case def A; }\n").unwrap();
    let path = file.to_str().unwrap().to_string();
    let parsed = parse_sysml_files(std::slice::from_ref(&path)).unwrap();
    std::fs::remove_dir_all(&folder).unwrap();
    let packages = parsed.workspace.packages();
    assert_eq!(
        packages.iter().map(|p| parsed.workspace.ast.text(*p, Prop::Name)).collect::<Vec<_>>(),
        [Some("UseCases")]
    );
    assert_eq!(parsed.warnings, [format!("{path} line 2: {LIBRARY_WARNING}")]);
}

#[test]
fn says_which_file_and_line_does_not_load() {
    let folder = std::env::temp_dir().join(format!("ortho-linking-errors-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("model.sysml");
    std::fs::write(&file, "package P {\n    part a : Missing;\n}\n").unwrap();
    let path = file.to_str().unwrap().to_string();
    let unresolved_name = parse_sysml_files(std::slice::from_ref(&path)).unwrap_err();
    std::fs::write(&file, "package P {\n    part a b;\n}\n").unwrap();
    let syntax = parse_sysml_files(std::slice::from_ref(&path)).unwrap_err();
    std::fs::remove_dir_all(&folder).unwrap();
    let expected = format!("Failed to parse:\n  {path} line 2: {}", unresolved("Definition", "Missing"));
    assert_eq!(unresolved_name, expected);
    assert_eq!(syntax, format!("Failed to parse:\n  {path} line 2: Expected ';' or '{{' but found 'b'."));
}

fn sysml_blocks(doc: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut rest = doc;
    while let Some(start) = rest.find("```sysml") {
        let body = rest[start + "```sysml".len()..].trim_start_matches('\r').strip_prefix('\n').unwrap();
        let end = body.find("```").unwrap();
        blocks.push(&body[..end]);
        rest = &body[end + 3..];
    }
    blocks
}

// Every model on LANGUAGE.md has to load, so the page never shows one that would not.
#[test]
fn language_page_holds_only_examples_that_parse_with_no_warnings_each_on_its_own() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let doc = std::fs::read_to_string(root.join("LANGUAGE.md")).unwrap();
    let blocks = sysml_blocks(&doc);
    assert!(!blocks.is_empty());
    for block in blocks {
        let workspace = build(block);
        let heading = block.lines().next().unwrap_or("");
        assert_eq!(reported(&workspace, Severity::Error), Vec::<String>::new(), "{heading}");
        assert_eq!(reported(&workspace, Severity::Warning), Vec::<String>::new(), "{heading}");
    }
}

#[test]
fn examples_load_with_no_warnings() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for example in ["coffee-machine", "survey-drone"] {
        let mut files: Vec<String> = std::fs::read_dir(root.join("examples").join(example))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "sysml"))
            .map(|path| path.to_str().unwrap().to_string())
            .collect();
        files.sort();
        assert_eq!(parse_sysml_files(&files).unwrap().warnings, Vec::<String>::new(), "{example}");
    }
}
