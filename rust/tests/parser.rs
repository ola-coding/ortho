//! What parsing alone decides. `oracle.rs` compares whole trees with Langium's.
use ortho::parser::ast::{Ast, NodeId, Prop, Type};
use ortho::parser::{parse, position};
use std::path::Path;

fn parse_ok(text: &str) -> (Ast, NodeId) {
    let mut ast = Ast::default();
    let model = parse(&mut ast, text).unwrap_or_else(|e| panic!("{}: {}", position(text, e.offset).0, e.message));
    (ast, model)
}

/// The line of a syntax error, and what it says.
fn syntax_error(text: &str) -> (usize, String) {
    let error = parse(&mut Ast::default(), text).expect_err("a syntax error");
    (position(text, error.offset).0, error.message)
}

fn member_named(ast: &Ast, owner: NodeId, name: &str) -> NodeId {
    ast.children(owner, Prop::Members).find(|m| ast.text(*m, Prop::Name) == Some(name)).unwrap()
}

fn first_package(ast: &Ast, model: NodeId) -> NodeId {
    ast.children(model, Prop::Packages).next().unwrap()
}

#[test]
fn parses_part_defs_with_attributes_ports_and_nested_parts() {
    let (ast, model) = parse_ok(
        "package P {
            port def PowerPort { attribute voltage : Real; }
            part def Battery {
                attribute capacity : Real = 2590;
                port powerOut : PowerPort;
                part cell : Cell [4];
            }
            part def Cell;
        }",
    );
    let battery = member_named(&ast, first_package(&ast, model), "Battery");
    assert_eq!(ast[battery].ty, Type::PartDef);
    let types: Vec<Type> = ast.children(battery, Prop::Members).map(|m| ast[m].ty).collect();
    assert_eq!(types, [Type::AttributeUsage, Type::PortUsage, Type::PartUsage]);
    let cell = member_named(&ast, battery, "cell");
    assert_eq!(ast.text(cell, Prop::Type), Some("Cell"));
    let multiplicity = ast.children(cell, Prop::Multiplicities).next().unwrap();
    assert_eq!(ast.text(multiplicity, Prop::Lower), Some("4"));
    assert_eq!(ast[cell].parent, Some(battery));
}

#[test]
fn keeps_what_no_view_draws_as_written() {
    let (ast, model) = parse_ok(
        "package P {
            requirement def MassLimit;
            requirement massReq : MassLimit;
            satisfy massReq by drone;
        }",
    );
    let pkg = first_package(&ast, model);
    let definition = member_named(&ast, pkg, "MassLimit");
    assert_eq!((ast[definition].ty, ast.text(definition, Prop::Kind)), (Type::OtherDefinition, Some("requirement")));
    let usage = member_named(&ast, pkg, "massReq");
    assert_eq!(ast.text(usage, Prop::Kind), Some("requirement"));
    assert_eq!(ast.texts(usage, Prop::Types).collect::<Vec<_>>(), ["MassLimit"]);
    let satisfy = ast.children(pkg, Prop::Members).find(|m| ast[*m].ty == Type::SatisfyUsage).unwrap();
    assert_eq!(ast.text(satisfy, Prop::Requirement), Some("massReq"));
    assert_eq!(ast.text(satisfy, Prop::Satisfier), Some("drone"));
}

#[test]
fn reads_a_quoted_name_without_its_quotes() {
    let (ast, model) = parse_ok(
        r"package 'Name Hardware' {
            part def 'Brew\tunit' { port 'water in' : 'Water port'; }
            part tank : 'Name Hardware'::Tank;
        }",
    );
    let pkg = first_package(&ast, model);
    assert_eq!(ast.text(pkg, Prop::Name), Some("Name Hardware"));
    let brew = member_named(&ast, pkg, "Brew\tunit");
    assert_eq!(ast.text(member_named(&ast, brew, "water in"), Prop::Type), Some("Water port"));
    assert_eq!(ast.text(member_named(&ast, pkg, "tank"), Prop::Type), Some("Name Hardware::Tank"));
}

#[test]
fn keeps_comments_in_the_model_and_notes_out_of_it() {
    let (ast, model) = parse_ok(
        r#"/* A comment on the whole file. */
        standard library package NameLibrary {
            doc /* Documentation of the package. */
            //* A note, which no tool keeps. */
            // Another note.
            comment about Engine /* About the engine. */
            part def Engine {
                doc <D1> locale "en_US" /* Documentation of the engine. */
                rep language "text" /* An engine. */
            }
        }
        library package NameOther {
            comment Note /* A named comment. */
        }"#,
    );
    assert_eq!(ast.children(model, Prop::Annotations).count(), 1);
    let library = first_package(&ast, model);
    assert!(ast.flag(library, Prop::IsStandard) && ast.flag(library, Prop::IsLibrary));
    let annotations: Vec<Type> = ast.children(library, Prop::Annotations).map(|a| ast[a].ty).collect();
    assert_eq!(annotations, [Type::Documentation, Type::Comment]);
    let engine = member_named(&ast, library, "Engine");
    let inner: Vec<Type> = ast.children(engine, Prop::Annotations).map(|a| ast[a].ty).collect();
    assert_eq!(inner, [Type::Documentation, Type::TextualRepresentation]);
}

#[test]
fn reads_every_kind_of_definition_and_usage() {
    parse_ok(
        "package Kinds {
            item def Fuel;
            enum def Color { red; green; enum blue; }
            abstract part def Vehicle {
                attribute mass : Real;
                ref item fuel : Fuel;
                part engine : Engine;
                port fuelIn : ~FuelPort;
            }
            part def Engine { event occurrence started; }
            port def FuelPort { in item fuel : Fuel; }
            part def Car :> Vehicle { part :>> engine : Engine; }
            connection def Hose { end part a : Engine; end part b : Engine; }
            allocation def Assign;
            state def Running;
            calc def Area;
            constraint def Limit;
            requirement def Speed { subject car : Car; }
            action def Drive { in fuel : Fuel; out distance : Real; }
            part car : Car {
                bind engine.started = engine.started;
                flow of Fuel from fuelIn to engine;
                exhibit state running : Running;
                #Safety part brake;
            }
            view def V;
            individual def Car1 :> Car;
            metadata def Safety;
            @Safety;
            dependency car to Car;
            occurrence def O { snapshot s; timeslice t; }
        }",
    );
}

#[test]
fn reads_values_and_expressions() {
    let (ast, model) = parse_ok(
        "package ExprKinds {
            private import ExprParts::*[@Safety];
            part def Car {
                attribute mass : Mass = 1500 [kg];
                attribute speed default := 3.6e1 * 2 ** 2;
                attribute fast = if speed > 100 ? true else false;
                attribute counts[*] = (1, 2, 3);
                attribute heavy = counts->select { in c; c > 1 }->size() >= 2 and not fast;
                constraint limit { mass <= 2000 [kg] }
                requirement r { require constraint { speed < 200 } }
            }
            calc def Area { in w : Real; in h : Real; return a : Real = w * h; }
            view v : GeneralView { filter @Safety; }
        }",
    );
    let car = member_named(&ast, first_package(&ast, model), "Car");
    let speed = member_named(&ast, car, "speed");
    assert!(ast.flag(speed, Prop::IsDefault) && ast.flag(speed, Prop::IsInitial));
    // `**` binds tighter than `*`.
    let times = ast.child(speed, Prop::Value).unwrap();
    assert_eq!(ast.text(times, Prop::Operator), Some("*"));
    assert_eq!(ast.text(ast.child(times, Prop::Right).unwrap(), Prop::Operator), Some("**"));
}

#[test]
fn requires_an_import_to_state_its_visibility() {
    assert!(parse(&mut Ast::default(), "package A { part def X; } package B { import A::*; }").is_err());
    for visibility in ["private", "protected", "public"] {
        parse_ok(&format!("package A {{ part def X; }} package B {{ {visibility} import A::*; }}"));
    }
}

#[test]
fn a_syntax_error_says_where_and_what_it_expected() {
    assert_eq!(syntax_error("package P {\n    part a b;\n}"), (2, "Expected ';' or '{' but found 'b'.".to_string()));
    assert_eq!(
        syntax_error("package P {\n    part a"),
        (2, "Expected ';' or '{' but found the end of the file.".to_string())
    );
    assert_eq!(syntax_error("package P {\n\n  part a : ;\n}"), (3, "Expected a name but found ';'.".to_string()));
    assert_eq!(
        syntax_error("package P {\n"),
        (2, "Expected a declaration or an expression but found the end of the file.".to_string())
    );
    assert_eq!(syntax_error("package é;"), (1, "Unexpected character 'é'.".to_string()));
    assert_eq!(syntax_error("package P;\n'open"), (2, "Unexpected character '''.".to_string()));
}

#[test]
fn a_file_that_does_not_parse_adds_nothing() {
    let mut ast = Ast::default();
    let model = parse(&mut ast, "package Kept;").unwrap();
    assert!(parse(&mut ast, "package Broken { part a b; }").is_err());
    assert_eq!(ast.text(first_package(&ast, model), Prop::Name), Some("Kept"));
    let another = parse(&mut ast, "package Next;").unwrap();
    assert_eq!(ast.text(first_package(&ast, another), Prop::Name), Some("Next"));
}

#[test]
fn counts_lines_and_characters_as_an_editor_does() {
    assert_eq!(position("a\nb", 2), (2, 1));
    assert_eq!(position("a\r\nb", 3), (2, 1));
    assert_eq!(position("a\rb", 2), (2, 1));
    assert_eq!(position("\n\nabc", 4), (3, 3));
    assert_eq!(position("\u{1F600}x", 4), (1, 3));
}

#[test]
fn reads_the_examples_and_fixtures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut read = 0;
    for folder in ["examples/coffee-machine", "examples/survey-drone", "tests/fixtures"] {
        for entry in std::fs::read_dir(root.join(folder)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "sysml") {
                parse_ok(&std::fs::read_to_string(path).unwrap());
                read += 1;
            }
        }
    }
    assert_eq!(read, 17);
}
