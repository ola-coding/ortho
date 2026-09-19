# The SysML v2 that ortho reads

ortho reads a small subset of the SysML v2 textual notation: what the six
views need, and a little more that models commonly carry. This page is that
subset, view by view, as a short version of the specification for anyone
writing models for ortho or working on its grammar.

It is written against **OMG Systems Modeling Language (SysML) v2.0, Part 1:
Language Specification, formal/2026-03-02** (March 2026),
<https://www.omg.org/spec/SysML/2.0/>. Section numbers (§) point into that
document. The wording here is ours; the specification is the authority.

**A model ortho accepts is meant to be valid SysML v2.** Where ortho cannot
check that, this page says so. Both example models, and every example on this
page, have been checked with the OMG pilot implementation (see
[How this is checked](#how-this-is-checked)).

[VIEWS.md](VIEWS.md) says what each view is for. This page says what you write
to get it.

## Rules for every file

**Packages and imports** (§7.5). Every file holds one or more `package`s.
An `import` always states its visibility (§7.5.3): write `private import`,
which is almost always what you mean. `public` and `protected` parse too.

```sysml
package Hardware {
    private import ScalarValues::*;

    part def Battery {
        attribute capacity_mAh : Real = 10000;
    }
}
```

- ortho does not enforce imports: every element is reachable by its qualified
  name. Write cross-file references qualified, `Software::sw.video`, and they
  resolve both here and in other tools.
- Attribute types are not resolved by ortho. `Real`, `String`, `Integer` and
  `Boolean` live in the library package `ScalarValues`, so import it, as above,
  or other tools will not find them.

`public` passes what it imports on to anyone who imports this package in turn.
`protected` means the same as `private` in a package, and differs only inside
a definition or usage.

```sysml
package Units {
    public import ScalarValues::Real;
    protected import ScalarValues::String;
}
```

**Definitions and usages** (§7.6). `part def Motor` defines a kind of part;
`part motor : Motor` is a use of one. A usage may carry a multiplicity,
`[4]`, `[1..3]` or `[0..*]`. A definition may specialize another with `:>` or
`specializes`, and inherits its parts, ports and connections.

```sysml
package Power {
    part def Battery;
    part def LiPoBattery :> Battery;
    part def LiIonBattery specializes Battery;

    part def Drone {
        part cells : LiPoBattery [1..2];
    }
}
```

**Qualified names and feature chains** (§7.6.6). `::` walks through packages;
`.` walks through features, and a chain has to start at a usage.
`Hardware::machine.controller` is right; `Hardware::CoffeeMachine::controller`,
which reaches into a definition, is not, and ortho rejects it. This is why the
examples end their hardware and software files with a single usage of the
whole, `part machine : CoffeeMachine` and `part sw { ... }`: it gives
references from other files somewhere to start.

**Names** (§8.2.2.1.2). SysML v2 reserves 128 keywords, and none of them can
be a name, even where ortho does not use the keyword itself. `frame`, `state`,
`item`, `flow` and `out` are the ones most often reached for. ortho rejects
them all.

**Comments.** `// ...` runs to the end of the line and `/* ... */` spans
lines. Both are ignored.

## Use case view

What people ask the system for, and who asks (§7.25).

```sysml
package UseCases {
    part def Operator;
    part def Technician;

    use case def OperateMachine {
        subject CoffeeMachine;
        actor operator : Operator;
    }

    use case makeEspresso : OperateMachine;

    use case makeCappuccino : OperateMachine {
        include use case makeEspresso;
    }

    use case descale : OperateMachine {
        subject CoffeeMachine;
        actor technician : Technician;
    }
}
```

| You write | The view draws |
|---|---|
| `use case name : Def;` | an ellipse. A `use case def` is drawn itself only when no usage is typed by it. |
| `subject Name;` in a use case (def) | the system boundary its use cases sit in, labelled with the subject's type, or its name when untyped |
| `actor name : Type;` | a stick figure joined to the use case. An actor on a `use case def` is primary and stands left; one added on a single use case is secondary and stands right. |
| `include use case name;` (or `include name;`) | a dashed «include» arrow. The target is always a use case usage, never a definition. |

The subject is a use case's first parameter, so a use case that declares an
actor states its `subject` before it, as `descale` does above, even though
`OperateMachine` already has one. ortho checks this.

## Logical view

What the system does, as a tree of functions (§7.17).

```sysml
package Functions {
    action def MakeBeverage {
        action prepareWater {
            action pumpWater;
            action heatWater;
        }
        action serveBeverage;
    }
}
```

| You write | The view draws |
|---|---|
| `action def Name { ... }` in a package | the root of a tree |
| `action name;` or `action name { ... }` nested in another | a box under its parent. Nesting is the decomposition. |

Nothing else in the file is drawn. Keep components out of this file: a
capability tree that names what realizes a function has stopped being
medium-independent.

## Implementation view

The software, as packages of modules and the dependencies between packages
(§7.5, §7.11).

```sysml
package Software {
    package Control {
        part def BrewController;
    }

    package Application {
        private import Software::Control::*;

        part def RecipeEngine;
    }

    part sw {
        part recipes : Application::RecipeEngine;
        part brew : Control::BrewController;
    }
}
```

| You write | The view draws |
|---|---|
| `package Name { ... }` | a package box, nested inside its parent package |
| `part def Name;` in a package | a module inside its package |
| `private import Pkg::*;` (or `Pkg::Element`) | an «import» dependency from the importing package to `Pkg` |
| `part sw { part x : Module; ... }` in a package | nothing. It is the tree of deployable parts the deployment view allocates from. |

## Physical view

The product as an internal block diagram: parts nested in the parts that
contain them, wired through their ports (§7.11–§7.14).

```sysml
package Hardware {
    port def WaterPort;
    port def DataPort;

    interface def ControlLink {
        end controller : DataPort;
        end device : DataPort;
    }

    part def Tank { port waterOut : WaterPort; }
    part def Pump {
        port waterIn : WaterPort;
        port ctrlIn : DataPort;
    }
    part def Board { port pumpCtrl : DataPort; }

    part def Machine {
        part tank : Tank;
        part pumps : Pump [2];
        part board : Board;

        connect tank.waterOut to pumps.waterIn;
        connection : ControlLink connect board.pumpCtrl to pumps.ctrlIn;
    }

    part machine : Machine;
}
```

| You write | The view draws |
|---|---|
| `part name : Def [n];` | a box for that part, `name : Def [n]`, with the parts of `Def` nested inside it |
| a `part def` nothing uses as a part type and nothing specializes, or a `part` usage directly in a package | a top-level box |
| `port name : PortDef;` | a port on the box's edge, labelled with its name |
| `connect a.p to b.q;` | a line between the two ports (or parts) |
| `connection : Interface connect a.p to b.q;` (optionally named) | the same line, labelled with the interface |
| `interface def` with its `end`s, `port def` | nothing of their own: they type ports and connections |
| `attribute name : Type = value;` | nothing. Values are specification, not topology. |

Parts and connections inherited through `:>` are drawn as if declared.

## Deployment view

Which computer runs which software (§7.15). The file declares nothing of its
own: it maps the software's usages onto the hardware's.

```sysml
package Software {
    part def RecipeEngine;
    part def UiApp;
    part sw {
        part recipes : RecipeEngine;
        part ui : UiApp;
    }
}

package Hardware {
    part def Board;
    part def Display;
    part def Machine {
        part controller : Board;
        part display : Display;
    }
    part machine : Machine;
}

package Deployment {
    allocate Software::sw.recipes to Hardware::machine.controller;
    allocation uiOnPanel allocate Software::sw.ui to Hardware::machine.display;
}
```

| You write | The view draws |
|---|---|
| `allocate software to host;` | the software drawn inside the host, a three-dimensional node |
| `allocation name allocate ... to ...;` | the same; the name is not drawn |
| (the host's place in the product) | each host inside a frame for the device it is fitted in |

Both ends are feature chains that start at a usage. Software allocated to two
hosts is drawn in the first only.

## Process view

One scenario: who sends what to whom, in order (§7.16, §7.17).

```sysml
package Scenario {
    part def RecipeEngine;
    part def BrewController;

    attribute def DoseSetting;

    action def Brew {
        part user;
        part recipes : RecipeEngine;
        part brew : BrewController;

        message order from user to recipes;
        then message grind of DoseSetting from recipes to brew;
        then message extract from brew to brew;
        then message finished from brew to recipes;
    }
}
```

| You write | The view draws |
|---|---|
| `part name : Def;` in the scenario's `action def` | a lifeline, once a message names it; lifelines run in order of first appearance |
| an untyped `part`, or one typed by an actor's type in a loaded use case | a lifeline headed by a stick figure |
| `message name from a to b;` | an arrow from `a` to `b`, in document order; `a` to `a` is a self-loop |
| `message name of Payload from a to b;` | the same arrow, labelled `name : Payload` |
| `attribute def Payload;` | nothing: it types a payload |
| `then` before a message | nothing more. It states the order the file already gives. |

## Parsed, not drawn

These parse, resolve and must be valid, but no view draws them.

**Requirements** (§7.21). A non-functional requirement is often what proposes
a technical solution on the implementation or physical view, so a model that
carries requirements has to keep loading.

```sysml
package Requirements {
    private import ScalarValues::*;

    part def Drone;

    requirement def Endurance {
        subject drone : Drone;
        attribute minutes : Real = 40;
    }

    requirement endurance : Endurance;

    part drone : Drone;
    satisfy endurance by drone;
}
```

`satisfy` names a requirement usage and the part usage that satisfies it,
never a definition.

**`perform`** (§7.17.6) inside a part names a use case usage the part
performs. Performing *functions* is how realization will be written: a part
that `perform`s an action from the logical view, drawn as a *perform actions*
compartment. That is on the backlog; today `perform` takes use cases only.

```sysml
package Operations {
    part def Pilot;

    use case def Fly {
        subject Aircraft;
        actor pilot : Pilot;
    }

    use case fly : Fly;

    part def Crew {
        perform fly;
    }
}
```

**Attribute values**, and the difference between `private`, `protected` and
`public` imports, parse but change nothing.

## Not supported

Common SysML v2 that ortho does not parse, so a model using it fails to load:

| Construct | § | Instead |
|---|---|---|
| `item def`, `item` | 7.10 | type a message payload with an `attribute def` |
| `flow`, succession flows | 7.16 | `connect` for topology, `message` for scenarios |
| `bind` | 7.13.3 | |
| `state def`, `state`, `exhibit` | 7.18 | |
| `allocation def` | 7.15.2 | the usage form, `allocate` or `allocation name allocate` |
| `in`, `out`, `inout` parameters | 7.17.2 | |
| `enum def` | 7.8 | |
| `calc`, `constraint` | 7.19–7.20 | |
| `view def`, `viewpoint` | 7.26 | one model file per view (see DEVELOPMENT.md) |
| `perform action` (realization) | 7.17.6 | not yet; see the backlog |
| `ref`, `abstract`, `redefines`/`:>>`, `subsets` | 7.6 | |
| units on values, `100 [kg]` | 7.7 | put the unit in the name, `mass_kg` |
| `doc`, `comment` elements | 7.4 | plain `//` and `/* */` comments |
| event occurrences as message ends | 7.9.5 | messages go from part to part |
| recursive and filtered imports, `::**`, `[...]` | 7.5.3–7.5.4 | `::*` or a single element |

## Keyword index

Every keyword ortho's grammar knows, and nothing else. A test keeps this table
and `grammar/sysml.langium` in step.

| Keyword | § | Used in |
|---|---|---|
| `action` | 7.17 | logical, process |
| `actor` | 7.25 | use case |
| `allocate` | 7.15 | deployment |
| `allocation` | 7.15 | deployment, to name an allocation |
| `attribute` | 7.7 | attribute usages; `attribute def` for message payloads |
| `by` | 7.21.4 | `satisfy ... by` |
| `case` | 7.25 | `use case` |
| `connect` | 7.13 | physical |
| `connection` | 7.13 | physical, to type or name a connection |
| `def` | 7.6 | every definition |
| `end` | 7.14 | interface ends |
| `from` | 7.16 | messages |
| `import` | 7.5.3 | every file that uses another |
| `include` | 7.25.3 | use case |
| `interface` | 7.14 | physical |
| `message` | 7.16 | process |
| `of` | 7.16 | message payloads |
| `package` | 7.5 | every file |
| `part` | 7.11 | physical, implementation, deployment, process |
| `perform` | 7.17.6 | parsed, not drawn |
| `port` | 7.12 | physical |
| `private` | 7.5.3 | imports |
| `protected` | 7.5.3 | imports |
| `public` | 7.5.3 | imports |
| `requirement` | 7.21 | parsed, not drawn |
| `satisfy` | 7.21.4 | parsed, not drawn |
| `specializes` | 7.6 | the long form of `:>` |
| `subject` | 7.25, 7.21 | use case boundaries; requirement subjects |
| `then` | 7.17.4 | message order |
| `to` | 7.13, 7.15, 7.16 | `connect`, `allocate`, `message` |
| `use` | 7.25 | `use case` |

## How this is checked

- **The grammar and this page agree.** A test compares the keyword index above
  with the keywords in `grammar/sysml.langium`, both ways.
- **Every example here parses.** A test parses each `sysml` block on this page
  on its own, and fails on any error.
- **Every keyword has a worked example**, in the example models or on this
  page. A test checks that each keyword appears in one of them.
- **The examples are valid SysML v2.** Both example models and every block on
  this page were run through the OMG pilot implementation, release 0.62.0
  (the `jupyter-sysml-kernel` package, with its bundled standard library, on
  Java 21), on 2026-09-19: no errors and no warnings. This is a one-off check,
  not part of CI; repeat it when the grammar grows.

The places where ortho knowingly differs from the specification are listed in
[DEVELOPMENT.md](DEVELOPMENT.md#deliberate-spec-deviations).
