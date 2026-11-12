//! Names, imports and what a model reports.
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
    // One with no name of its own goes by the name it takes on, or by none.
    let unnamed = |declaration: &str| {
        format!(
            "package U {{
                part def Operator;
                use case def Operate {{ subject Machine; actor operator : Operator; use case service; }}
                use case def Serve :> Operate {{ {declaration} {{ actor technician : Operator; }} }}
            }}"
        )
    };
    assert_eq!(
        errors_of(&unnamed("use case :>> service")),
        ["Declare the subject of 'service' before its actors: the subject is a use case's first parameter."]
    );
    assert_eq!(
        errors_of(&unnamed("use case : Operate")),
        ["Declare the use case's subject before its actors: the subject is a use case's first parameter."]
    );
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

fn unperformed(name: &str) -> String {
    format!(
        "No part or module of the implementation or physical view performs '{name}'. Perform it, or a function above it, in the one that realizes it."
    )
}

/// A function tree a logical view draws, `solution` beside it, and the part
/// definitions in `hardware`, which may perform its functions.
fn realized_with(solution: &str, hardware: &str) -> String {
    format!(
        "package Functions {{
            private import StandardViewDefinitions::*;
            action def MakeBeverage {{
                action prepareWater {{ action storeWater; action heatWater; }}
                action prepareCoffee {{ action grindBeans; }}
            }}
            action beverage : MakeBeverage;
            view logical : GeneralView {{ expose MakeBeverage; }}
            {solution}
        }}
        package Hardware {{
            private import Functions::*;
            {hardware}
        }}"
    )
}

/// The same, with an implementation view that draws the definitions in `hardware` as modules.
fn realized(hardware: &str) -> String {
    realized_with("view modules : GeneralView { expose Hardware; }", hardware)
}

#[test]
fn warns_of_a_function_no_module_or_part_of_the_solution_performs() {
    // What is left once an agent deletes the grinder's perform to silence an error.
    let workspace = build(&realized(
        "part def Tank { perform beverage.prepareWater.storeWater; }
         part def Heater { perform beverage.prepareWater.heatWater; }
         part def Grinder;",
    ));
    assert_eq!(reported(&workspace, Severity::Warning), [unperformed("grindBeans")]);
    assert_eq!(reported(&workspace, Severity::Error), Vec::<String>::new());
    // Every function needs a solution, not only once others have one.
    let none = build(&realized("part def Tank; part def Heater; part def Grinder;"));
    assert_eq!(
        reported(&none, Severity::Warning),
        [unperformed("storeWater"), unperformed("heatWater"), unperformed("grindBeans")]
    );
}

#[test]
fn a_model_without_an_implementation_or_physical_view_needs_no_performs() {
    let workspace = build(&realized_with("", "part def Tank; part def Heater; part def Grinder;"));
    assert_eq!(reported(&workspace, Severity::Warning), Vec::<String>::new());
}

#[test]
fn a_perform_counts_only_where_a_solution_view_draws_it() {
    let performs = "part def Tank { perform beverage.prepareWater.storeWater; }
                    part def Heater { perform beverage.prepareWater.heatWater; }";
    // A module in a package the implementation view does not expose.
    let elsewhere = realized_with(
        "view modules : GeneralView { expose Hardware; }
         package Spare { part def Grinder { perform beverage.prepareCoffee.grindBeans; } }",
        performs,
    );
    assert_eq!(reported(&build(&elsewhere), Severity::Warning), [unperformed("grindBeans")]);
    // A part the physical view draws, nested or not.
    let physical = |nested: &str| {
        realized_with(
            "view physical : InterconnectionView { expose Hardware::machine; }",
            &format!(
                "{performs}
                 part def Grinder {{ perform beverage.prepareCoffee.grindBeans; }}
                 part def Machine {{ part tank : Tank; part heater : Heater; {nested} }}
                 part machine : Machine;"
            ),
        )
    };
    assert_eq!(reported(&build(&physical("")), Severity::Warning), [unperformed("grindBeans")]);
    assert_eq!(reported(&build(&physical("part grinder : Grinder;")), Severity::Warning), Vec::<String>::new());
}

#[test]
fn in_a_model_with_use_cases_only_a_tree_that_refines_one_needs_a_solution() {
    let text = |use_case_view: &str| {
        format!(
            "package SystemUseCases {{ use case def Operate; }}
            package Functions {{
                private import StandardViewDefinitions::*;
                private import ModelingMetadata::*;
                action def MakeBeverage {{ action heatWater; }}
                action def Clean {{ action rinse; }}
                action beverage : MakeBeverage;
                #refinement dependency MakeBeverage to SystemUseCases::Operate;
                view logical : GeneralView {{ expose MakeBeverage; expose Clean; }}
                view modules : GeneralView {{ expose Functions; }}
                {use_case_view}
                part def Heater {{ perform beverage.heatWater; }}
            }}"
        )
    };
    // Clean, a catalogue tree no use case asks for, needs no solution yet.
    let scoped = build(&text("view useCases : GeneralView { expose SystemUseCases; }"));
    assert_eq!(reported(&scoped, Severity::Warning), Vec::<String>::new());
    assert_eq!(reported(&build(&text("")), Severity::Warning), [unperformed("rinse")]);
}

#[test]
fn performing_a_function_performs_the_functions_under_it() {
    for hardware in [
        "part def Boiler { perform beverage.prepareWater; }
         part def Grinder { perform beverage.prepareCoffee.grindBeans; }",
        // A typed action stands for its definition's whole tree.
        "part def Machine { perform beverage; }",
    ] {
        assert_eq!(reported(&build(&realized(hardware)), Severity::Warning), Vec::<String>::new(), "{hardware}");
    }
}

#[test]
fn only_the_functions_a_logical_view_draws_have_to_be_performed() {
    let text = |exposed: &str| {
        format!(
            "package Functions {{
                private import StandardViewDefinitions::*;
                action def MakeBeverage {{ action heatWater; action grindBeans; }}
                action def Clean {{ action rinse; }}
                action beverage : MakeBeverage;
                view logical : GeneralView {{ {exposed} }}
                view modules : GeneralView {{ expose Functions; }}
                part def Heater {{ perform beverage.heatWater; }}
                part def Grinder {{ perform beverage.grindBeans; }}
            }}"
        )
    };
    assert_eq!(reported(&build(&text("expose MakeBeverage;")), Severity::Warning), Vec::<String>::new());
    assert_eq!(
        reported(&build(&text("expose MakeBeverage; expose Clean;")), Severity::Warning),
        [unperformed("rinse")]
    );
}

fn unrefined(name: &str) -> String {
    format!(
        "'{name}' refines no use case, though a logical view draws it beside use cases. Add a #refinement dependency from it to the use case it details."
    )
}

/// A function tree, a use case, and `refinement` beside them, with a logical view exposing `exposed`.
fn refining(exposed: &str, refinement: &str) -> String {
    format!(
        "package SystemUseCases {{ use case def OperateMachine; }}
        package Functions {{
            private import StandardViewDefinitions::*;
            private import ModelingMetadata::*;
            action def MakeBeverage {{ action prepareWater {{ action heatWater; }} }}
            action beverage : MakeBeverage;
            {refinement}
            view logical : GeneralView {{ {exposed} }}
        }}"
    )
}

#[test]
fn warns_of_a_tree_that_refines_no_use_case_on_a_view_that_shows_use_cases() {
    let workspace = build(&refining("expose MakeBeverage; expose SystemUseCases::OperateMachine;", ""));
    assert_eq!(
        reported(&workspace, Severity::Warning),
        [unrefined_use_case("OperateMachine"), unrefined("MakeBeverage")]
    );
    assert_eq!(reported(&workspace, Severity::Error), Vec::<String>::new());
}

#[test]
fn a_tree_on_a_view_without_use_cases_needs_no_refinement_as_it_is_optional() {
    let workspace = build(&refining("expose MakeBeverage;", ""));
    assert_eq!(reported(&workspace, Severity::Warning), Vec::<String>::new());
}

#[test]
fn a_refinement_from_the_tree_or_a_use_of_it_refines_the_tree_also_on_a_view_of_a_branch() {
    for refinement in [
        "#refinement dependency MakeBeverage to SystemUseCases::OperateMachine;",
        "#refinement dependency beverage to SystemUseCases::OperateMachine;",
    ] {
        for exposed in ["expose MakeBeverage;", "expose MakeBeverage::prepareWater;"] {
            let text = refining(&format!("{exposed} expose SystemUseCases::OperateMachine;"), refinement);
            assert_eq!(reported(&build(&text), Severity::Warning), Vec::<String>::new(), "{refinement} {exposed}");
        }
    }
    // A branch whose tree refines nothing: the warning is at the tree.
    let branch = refining("expose MakeBeverage::prepareWater; expose SystemUseCases::OperateMachine;", "");
    assert_eq!(
        reported(&build(&branch), Severity::Warning),
        [unrefined_use_case("OperateMachine"), unrefined("MakeBeverage")]
    );
}

fn unrefined_use_case(name: &str) -> String {
    format!(
        "No function tree refines '{name}', though a logical view draws it beside trees. Add a #refinement dependency to it from the tree that details it."
    )
}

#[test]
fn warns_of_a_use_case_no_tree_refines_on_a_logical_view_that_draws_trees() {
    let text = |exposed: &str| {
        format!(
            "package SystemUseCases {{ use case def OperateMachine; use case def ServiceMachine; }}
            package Functions {{
                private import StandardViewDefinitions::*;
                private import ModelingMetadata::*;
                action def MakeBeverage {{ action heatWater; }}
                #refinement dependency MakeBeverage to SystemUseCases::OperateMachine;
                view logical : GeneralView {{ {exposed} }}
            }}"
        )
    };
    let both = "expose MakeBeverage; expose SystemUseCases::OperateMachine; expose SystemUseCases::ServiceMachine;";
    assert_eq!(reported(&build(&text(both)), Severity::Warning), [unrefined_use_case("ServiceMachine")]);
    // A use case the logical view does not draw asks nothing of it.
    let one = "expose MakeBeverage; expose SystemUseCases::OperateMachine;";
    assert_eq!(reported(&build(&text(one)), Severity::Warning), Vec::<String>::new());
}

/// A function tree, and a part that performs `chain` in it.
fn performing(chain: &str) -> Vec<String> {
    errors_of(&format!(
        "package Functions {{
            action def MakeBeverage {{
                action prepareCoffee {{ action storeMrBeans; action grindBeans; action doseGrounds; }}
            }}
            action beverage : MakeBeverage;
            part def BeanHopper {{ perform {chain}; }}
        }}"
    ))
}

#[test]
fn suggests_the_name_meant_in_a_chain_from_what_the_segment_before_has() {
    // A rename: the old name is two letters from the new one, found through beverage's type.
    assert_eq!(
        performing("beverage.prepareCoffee.storeBeans"),
        [format!("{} Did you mean 'storeMrBeans'?", unresolved("Feature", "storeBeans"))]
    );
    // Case does not count, and a tie names both.
    assert_eq!(
        performing("beverage.prepareCoffee.GRINDBEANS"),
        [format!("{} Did you mean 'grindBeans'?", unresolved("Feature", "GRINDBEANS"))]
    );
    let tied = errors_of(
        "package P {
            action def A { action stepA; action stepB; }
            action a : A;
            part def X { perform a.stepC; }
        }",
    );
    assert_eq!(tied, [format!("{} Did you mean 'stepA' or 'stepB'?", unresolved("Feature", "stepC"))]);
}

#[test]
fn lists_what_the_segment_before_has_when_no_name_is_close() {
    assert_eq!(
        performing("beverage.prepareCoffee.fillHopper"),
        [format!("{} 'prepareCoffee' has storeMrBeans, grindBeans, doseGrounds.", unresolved("Feature", "fillHopper"))]
    );
    let many = errors_of(
        "package P {
            action def A { action a1; action a2; action a3; action a4; action a5; action a6; action a7; action a8; action a9; action a10; }
            action a : A;
            part def X { perform a.unrelatedName; }
        }",
    );
    assert_eq!(
        many,
        [format!("{} 'a' has a1, a2, a3, a4, a5, a6, a7, a8 and 2 more.", unresolved("Feature", "unrelatedName"))]
    );
}

#[test]
fn gives_no_hint_for_a_name_found_through_packages_and_imports() {
    // A chain's first segment, and a type, are looked up through packages and imports.
    assert_eq!(performing("bevrage.prepareCoffee"), [unresolved("Feature", "bevrage")]);
    assert_eq!(errors_of("package P { part def Board; part b : Bord; }"), [unresolved("Definition", "Bord")]);
}

fn foreign(end: &str, problem: &str) -> String {
    format!("'{end}' {problem}. A deployment view allocates what the other views draw, and adds nothing of its own.")
}

const NOT_A_MODULE: &str = "is not a module the implementation view draws";
const NOT_A_PART: &str = "is not a part the physical view draws";

/// Software, hardware, and a deployment view of `allocations`, beside `views`. The
/// deployment package declares a program and a board of its own, out of thin air.
fn deployed(views: &str, allocations: &str) -> String {
    format!(
        "package Software {{
            part def RecipeEngine;
            part def UiApp {{ part engine : RecipeEngine; }}
            part sw {{ part recipeEngine : RecipeEngine; part uiApp : UiApp; }}
        }}
        package Hardware {{
            port def Bus;
            part def Board {{ port bus : Bus; }}
            part def Display;
            part def Machine {{ part controller : Board; part display : Display; }}
            part machine : Machine;
        }}
        package Deployment {{
            private import Software::*;
            private import Hardware::*;
            private import StandardViewDefinitions::*;
            part ghostApp;
            part spareBoard;
            {allocations}
            view deployment : GeneralView {{ expose Deployment; }}
            {views}
        }}"
    )
}

const BOTH_VIEWS: &str = "view modules : GeneralView { expose Software; }
                          view physical : InterconnectionView { expose machine; }";

#[test]
fn warns_of_an_allocate_end_that_no_other_view_draws() {
    let workspace = build(&deployed(
        BOTH_VIEWS,
        "allocate sw.recipeEngine to machine.controller;
         allocate ghostApp to machine.display;
         allocate sw.uiApp to spareBoard;",
    ));
    assert_eq!(
        reported(&workspace, Severity::Warning),
        [foreign("ghostApp", NOT_A_MODULE), foreign("spareBoard", NOT_A_PART)]
    );
    // A port stands for the part it is on.
    let port = build(&deployed(
        BOTH_VIEWS,
        "allocate sw.recipeEngine to machine.controller.bus; allocate sw.uiApp to machine.display;",
    ));
    assert_eq!(reported(&port, Severity::Warning), Vec::<String>::new());
}

#[test]
fn checks_an_allocate_end_only_once_the_model_declares_the_view_it_comes_from() {
    let allocations = "allocate ghostApp to spareBoard;
                       allocate sw.recipeEngine to machine.controller;
                       allocate sw.uiApp to machine.display;";
    let modules_only = build(&deployed("view modules : GeneralView { expose Software; }", allocations));
    assert_eq!(reported(&modules_only, Severity::Warning), [foreign("ghostApp", NOT_A_MODULE)]);
    let physical_only = build(&deployed("view physical : InterconnectionView { expose machine; }", allocations));
    assert_eq!(reported(&physical_only, Severity::Warning), [foreign("spareBoard", NOT_A_PART)]);
    assert_eq!(reported(&build(&deployed("", allocations)), Severity::Warning), Vec::<String>::new());
}

#[test]
fn refuses_a_part_or_an_actor_typed_by_behaviour_or_data() {
    let typed = |usage: &str, definition: &str| {
        errors_of(&format!("package P {{ {definition} D; part def Owner {{ {usage} x : D; }} }}"))
    };
    assert_eq!(typed("part", "action def"), ["'D' is an action definition. A part is typed by a part definition."]);
    assert_eq!(
        typed("actor", "use case def"),
        ["'D' is a use case definition. An actor is typed by a part definition."]
    );
    for definition in ["attribute def", "enum def", "occurrence def", "port def"] {
        assert_eq!(typed("part", definition).len(), 1, "{definition}");
    }
    // A part definition's kinds, and an item definition, which a part definition specializes.
    for definition in ["part def", "interface def", "connection def", "allocation def", "item def"] {
        assert_eq!(typed("part", definition), Vec::<String>::new(), "{definition}");
    }
}

fn unhosted(name: &str) -> String {
    format!(
        "'{name}' runs on no host: no deployment view allocates it. Allocate it to the part of the physical view that runs it."
    )
}

#[test]
fn warns_of_software_that_no_deployment_view_allocates_to_a_host() {
    let one = "allocate sw.recipeEngine to machine.controller;";
    // UiApp's own engine runs where uiApp runs, so only uiApp is missing.
    assert_eq!(reported(&build(&deployed(BOTH_VIEWS, one)), Severity::Warning), [unhosted("uiApp")]);
    // Without an implementation view there is no software to account for.
    let physical_only = deployed("view physical : InterconnectionView { expose machine; }", one);
    assert_eq!(reported(&build(&physical_only), Severity::Warning), Vec::<String>::new());
}

/// The coffee machine's warnings once `deleted` is gone from its files, as an agent
/// may delete the line an error points at, and its logical.sysml as named in them.
fn coffee_machine_without(deleted: &str, folder: &str) -> (Vec<String>, String) {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/coffee-machine");
    let folder = std::env::temp_dir().join(format!("ortho-linking-{folder}-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&example).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "sysml") {
            let copy = folder.join(path.file_name().unwrap());
            std::fs::write(&copy, std::fs::read_to_string(&path).unwrap().replace(deleted, "")).unwrap();
            files.push(copy.to_str().unwrap().to_string());
        }
    }
    files.sort();
    let parsed = parse_sysml_files(&files);
    std::fs::remove_dir_all(&folder).unwrap();
    (parsed.unwrap().warnings, folder.join("logical.sysml").to_str().unwrap().to_string())
}

/// The line, from 1, of the coffee machine's logical.sysml that reads `written`.
fn coffee_machine_logical_line(written: &str) -> usize {
    coffee_machine_line("logical.sysml", written)
}

/// The line, from 1, of one of the coffee machine's files that reads `written`.
fn coffee_machine_line(file: &str, written: &str) -> usize {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(root.join("examples/coffee-machine").join(file)).unwrap();
    text.lines().position(|line| line.trim() == written).unwrap() + 1
}

#[test]
fn warns_at_the_function_when_the_coffee_machine_loses_a_perform() {
    // The bean hopper's, on the physical view.
    let (warnings, logical) = coffee_machine_without("perform beverage.prepareCoffee.storeBeans;", "unperformed");
    let line = coffee_machine_logical_line("action storeBeans;");
    assert_eq!(warnings, [format!("{logical} line {line}: {}", unperformed("storeBeans"))]);
}

#[test]
fn warns_at_the_tree_when_the_coffee_machine_loses_a_refinement() {
    let refinement = "#refinement dependency MaintainMachine to SystemUseCases::descale;";
    let (warnings, logical) = coffee_machine_without(refinement, "unrefined");
    let line = coffee_machine_logical_line("action def MaintainMachine {");
    // Both ends of the lost refinement: the tree, and the use case it detailed.
    let use_cases = logical.replace("logical.sysml", "use-case.sysml");
    let use_case_line = coffee_machine_line("use-case.sysml", "use case descale : OperateMachine {");
    assert_eq!(
        warnings,
        [
            format!("{logical} line {line}: {}", unrefined("MaintainMachine")),
            format!("{use_cases} line {use_case_line}: {}", unrefined_use_case("descale")),
        ]
    );
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
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
