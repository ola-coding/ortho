//! The table of requirements: a GridView as Markdown, and what `satisfy` is checked for.
use ortho::diagrams::requirements::comment_paragraphs;
use ortho::model::views::{DiagramKind, diagram_kind};
use ortho::parser::workspace::{Severity, Workspace};
use ortho::pipeline::render_views;
use ortho::render::theme::LIGHT;

const REQUIREMENTS: &str = "package CoffeeMachineRequirements {
    private import ModelingMetadata::*;
    private import StandardViewDefinitions::*;

    requirement <'REQ-001'> brewTime {
        doc /* The machine shall deliver an espresso
             * within 30 s of the user's choice.
             *
             * Timed from the press of a button. */
        @Rationale { text = \"Users compare the wait with a café's.\"; }
        requirement <'REQ-002'> heatUp {
            doc /* The water shall reach 92 °C within 25 s from cold. */
        }
        requirement <'REQ-003'> extraction {
            doc /* The water shall reach the brew unit at 9 bar. */
        }
    }
    requirement <'REQ-004'> dryRunCutoff {
        doc /* The heater shall switch off within 2 s when the tank runs dry. */
        @Rationale { text = \"A heater running dry can crack.\"; }
    }
    requirement <'REQ-005'> beverageChoice {
        doc /* The user shall choose a beverage on a touch display. */
    }

    view 'Requirements — Coffee Machine' : GridView {
        doc /* What the coffee machine shall do. */
        expose CoffeeMachineRequirements::**;
    }
}";

const PHYSICAL: &str = "package Physical {
    private import StandardViewDefinitions::*;
    part def Display { satisfy CoffeeMachineRequirements::beverageChoice; }
    part def CoffeeMachine {
        part heater;
        part pump;
        part display : Display;
    }
    part machine : CoffeeMachine;
    satisfy CoffeeMachineRequirements::brewTime by machine.heater;
    satisfy CoffeeMachineRequirements::brewTime by machine.pump;
    satisfy CoffeeMachineRequirements::brewTime.heatUp by machine.heater;
    view 'Physical' : InterconnectionView { expose machine; }
}";

fn build(files: &[(&str, &str)]) -> Workspace {
    let sources = files.iter().map(|(path, text)| (path.to_string(), text.to_string())).collect();
    Workspace::build(sources).unwrap_or_else(|failures| panic!("{failures:?}"))
}

fn reported(workspace: &Workspace, severity: Severity) -> Vec<String> {
    let all = workspace.documents.iter().flat_map(|document| &document.diagnostics);
    all.filter(|d| d.severity == severity).map(|d| d.message.clone()).collect()
}

#[test]
fn writes_a_grid_view_of_requirements_as_a_markdown_specification() {
    let workspace = build(&[("spec/requirements.sysml", REQUIREMENTS), ("spec/physical.sysml", PHYSICAL)]);
    let rendering = render_views(&workspace, &LIGHT, "spec/diagrams").unwrap();
    let table = &rendering.rendered[0];
    assert_eq!(diagram_kind(&workspace.ast, table.view), DiagramKind::Requirements);
    assert_eq!(table.file, "requirements-coffee-machine.md");
    assert_eq!(table.result.summary, "5 requirement(s), 4 satisfied");
    assert_eq!(table.warning, None);
    let expected = "# Requirements — Coffee Machine

<sub>Rendered by ortho from [requirements.sysml](../requirements.sysml). Edit the model, not this file.</sub>

What the coffee machine shall do.

5 requirements, 4 satisfied. Not satisfied: [REQ-004](#dryruncutoff).

## brewTime

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-001 | The machine shall deliver an espresso within 30 s of the user's choice.<br><br>Timed from the press of a button. | Users compare the wait with a café's. |

### heatUp

| ID | Statement |
|:---|:---|
| REQ-002 | The water shall reach 92 °C within 25 s from cold. |

### extraction

| ID | Statement |
|:---|:---|
| REQ-003 | The water shall reach the brew unit at 9 bar. |

## dryRunCutoff

| ID | Statement | Rationale |
|:---|:---|:---|
| REQ-004 | The heater shall switch off within 2 s when the tank runs dry. | A heater running dry can crack. |

## beverageChoice

| ID | Statement |
|:---|:---|
| REQ-005 | The user shall choose a beverage on a touch display. |

## Traceability

| ID | Requirement | Satisfied by |
|:---|:---|:---|
| [REQ-001](#brewtime) | brewTime | [machine.heater](../physical.sysml#L5), [machine.pump](../physical.sysml#L6) |
| [REQ-002](#heatup) | &emsp;heatUp | [machine.heater](../physical.sysml#L5) |
| [REQ-003](#extraction) | &emsp;extraction | through [REQ-001](#brewtime) |
| [REQ-004](#dryruncutoff) | dryRunCutoff | — |
| [REQ-005](#beveragechoice) | beverageChoice | [Display](../physical.sysml#L3) |
";
    assert_eq!(table.result.content, expected);
    // Written first, before the diagrams: what is asked of the system comes first.
    assert_eq!(rendering.rendered[1].file, "physical.svg");
}

#[test]
fn warns_of_a_requirement_nothing_satisfies_once_the_model_has_a_solution_view() {
    let workspace = build(&[("spec/requirements.sysml", REQUIREMENTS), ("spec/physical.sysml", PHYSICAL)]);
    assert_eq!(reported(&workspace, Severity::Error), Vec::<String>::new());
    assert_eq!(
        reported(&workspace, Severity::Warning),
        [
            "No part or module of the implementation or physical view satisfies 'dryRunCutoff'. Satisfy it, or a requirement it is in, by the one that meets it."
        ]
    );
    // Without a solution view, the requirements wait for one.
    let alone = build(&[("spec/requirements.sysml", REQUIREMENTS)]);
    assert_eq!(reported(&alone, Severity::Warning), Vec::<String>::new());
}

#[test]
fn warns_of_a_requirement_without_an_id_or_with_the_id_of_another() {
    let workspace = build(&[(
        "model.sysml",
        "package Spec {
            requirement <'REQ-001'> a;
            requirement b;
            requirement <'REQ-001'> c;
            requirement <'REQ-001'> notListed;
            view table : GridView { expose Spec::a; expose Spec::b; expose Spec::c; }
        }",
    )]);
    assert_eq!(
        reported(&workspace, Severity::Warning),
        [
            "'b' has no ID for the table of requirements to list it by. Give it one as its short name, as in requirement <'REQ-001'> b.",
            "'REQ-001' is the ID of 'a' too. Give each requirement an ID of its own.",
        ]
    );
}

#[test]
fn says_what_a_satisfy_meant_when_it_names_nothing_or_no_requirement() {
    let workspace = build(&[
        ("spec/requirements.sysml", REQUIREMENTS),
        (
            "spec/physical.sysml",
            "package Physical {
                part def CoffeeMachine { part heater; part pump; }
                part machine : CoffeeMachine;
                satisfy CoffeeMachineRequirements::brewTym by machine.heater;
                satisfy CoffeeMachineRequirements::heatUp by machine.heater;
                satisfy CoffeeMachineRequirements::brewTime by machine.pmp;
                satisfy machine by machine.pump;
            }",
        ),
    ]);
    assert_eq!(
        reported(&workspace, Severity::Error),
        [
            "Could not resolve reference to Feature named 'CoffeeMachineRequirements::brewTym'. Did you mean 'CoffeeMachineRequirements::brewTime'?",
            "Could not resolve reference to Feature named 'CoffeeMachineRequirements::heatUp'. Did you mean 'CoffeeMachineRequirements::brewTime.heatUp'?",
            "Could not resolve reference to Feature named 'pmp'. Did you mean 'pump'?",
            "'machine' is not a requirement, so it cannot be satisfied.",
        ]
    );
}

#[test]
fn skips_a_grid_view_of_anything_but_requirements() {
    let workspace =
        build(&[("model.sysml", "package P { part def A; part a : A; view parts : GridView { expose a; } }")]);
    let rendering = render_views(&workspace, &LIGHT, "diagrams").unwrap();
    assert!(rendering.rendered.is_empty());
    assert_eq!(rendering.skipped[0].reason, "A GridView exposes packages or requirements.");
}

#[test]
fn reads_a_comment_body_as_paragraphs() {
    assert_eq!(comment_paragraphs("/* One line. */"), ["One line."]);
    assert_eq!(
        comment_paragraphs("/*\n * First,\n * continued.\n *\n * Second.\n */"),
        ["First, continued.", "Second."]
    );
}

#[test]
fn escapes_what_markdown_would_read_as_markup() {
    let workspace = build(&[(
        "my model.sysml",
        "package P {
            requirement <'R_1'> 'pipe|and*star' { doc /* 1. Use <b>no</b> | markup. */ }
            view table : GridView { expose P::*; }
        }",
    )]);
    let content = &render_views(&workspace, &LIGHT, ".").unwrap().rendered[0].result.content;
    assert!(content.contains("from [my model.sysml](my%20model.sysml)."), "{content}");
    assert!(
        content.contains(
            "## pipe\\|and\\*star\n\n| ID | Statement |\n|:---|:---|\n| R\\_1 | 1. Use \\<b\\>no\\</b\\> \\| markup. |\n"
        ),
        "{content}"
    );
    assert!(content.contains("| [R\\_1](#pipeandstar) | pipe\\|and\\*star | — |"), "{content}");
}
