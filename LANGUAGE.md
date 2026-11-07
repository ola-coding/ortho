# The SysML v2 that ortho reads

ortho reads the whole SysML v2 textual notation and draws six views from it.
This page says, view by view, what each view draws from a model. Section
numbers (§) point into **OMG SysML v2.0, Part 1, formal/2026-03-02**,
<https://www.omg.org/spec/SysML/2.0/>, which is the authority.

Where a view draws, ortho checks a model as SysML v2 does; elsewhere, only
its syntax ([how this is checked](#how-this-is-checked)). What each view is
for is in [VIEWS.md](VIEWS.md).

## Rules for every file

**Packages and imports** (§7.5). A file holds one or more `package`s. An
`import` states its visibility (§7.5.3), and `private import` is almost
always the right one.

```sysml
package Hardware {
    private import ScalarValues::*;

    part def Battery {
        attribute capacity_mAh : Real = 10000;
    }
}
```

`public` passes an import on to whoever imports this package; `protected`
acts like `private` in a package.

```sysml
package Units {
    public import ScalarValues::Real;
    protected import ScalarValues::String;
}
```

- **Imports make names visible.** After `private import Software::*;`,
  `sw.videoStreamer` is `Software::sw.videoStreamer`. A package's own members
  come before what it imports.
- **Attribute types are not resolved.** Import `ScalarValues`, where `Real`,
  `String`, `Integer` and `Boolean` live, so that other tools find them.

**Definitions and usages** (§7.6). `part def Motor` defines a kind of part;
`part motor : Motor` uses one, optionally with a multiplicity: `[4]`,
`[1..3]`, `[0..*]`. A definition specializes another with `:>` or
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

**Feature chains** (§7.6.6). `::` walks through packages and `.` through
features, and a chain starts at a usage: `Hardware::machine.controller`, not
`Hardware::CoffeeMachine::controller`. So a file that others refer to declares
one usage of each whole for them to start from: `part sw { ... }`,
`part machine : CoffeeMachine` or `action beverage : MakeBeverage`.

**Names** (§8.2.2.1.2). A name in single quotes may hold anything, as in
`'Brew unit'`, and is the same name without them. An unquoted name may not be
one of SysML v2's 128 reserved words, even those ortho does not use, such as
`frame`, `state`, `item`, `flow` and `out`. A short name in angle brackets,
`part def <BU> 'Brew unit';`, names the element too, as does an `alias`. Name
no top-level package after a standard library package, such as `UseCases`,
`Requirements` or `Views`: it hides the library's, and other tools may find the
library's instead. ortho warns of one, and renders anyway.

**Typing** (§8.3). A part is typed by a part definition, and so is an actor,
which is a part: `part controller : Board`, not `part controller : Brew`
where `Brew` is an `action def`. An interface, connection or allocation
definition is one kind of part definition, and an item definition, which a
part definition specializes, is accepted too. A part typed by behaviour or
data, an action, use case, occurrence, enum or attribute definition, or by a
port definition, is an error. A port, an action and a use case can only name
their own kind of definition at all.

**Comments** (§8.2.2.4). `// ...` and `//* ... */` are notes, which no tool
keeps. `/* ... */`, `doc /* ... */` and `comment about x /* ... */` are part of
the model; no view draws them.

## Views

A model declares its views (§7.26), and ortho draws each: a `view` typed by
one of the standard view definitions, exposing what the diagram shows. Write it
after the model, as OMG's models do. Its name is the diagram's heading
(§7.26.5), in single quotes unless it is a plain name: the frame's tab reads
«view» and the name (§8.2.3.26). ortho matches the definition by name, so import
`StandardViewDefinitions` for other tools.

A view may also name how it is rendered (§7.26.2). The logical view says
`render asTreeDiagram;`, the standard library's tree rendering, imported from
`Views`. ortho draws a function tree as a tree either way; the line asks other
tools to do the same.

```sysml
package Functions {
    private import StandardViewDefinitions::*;
    private import Views::*;

    action def MakeBeverage {
        action heatWater;
    }

    view 'Logical view — Coffee Machine' : GeneralView {
        expose MakeBeverage;
        render asTreeDiagram;
    }
}
```

| View | Definition | Exposes |
|---|---|---|
| use case | `GeneralView` | the package of use cases |
| logical | `GeneralView` | the `action def` of each function tree, and the use cases they refine |
| implementation | `GeneralView` | the package of software |
| physical | `InterconnectionView` | the product's `part` usage |
| deployment | `GeneralView` | the package of allocations |
| process | `SequenceView` | the scenario's `occurrence def` |

A view draws only what it exposes, and a `GeneralView` draws whatever that is.
ortho skips any other view a model declares, and says why; with nothing left to
draw, it fails.

### Keeping a view on one page

ortho draws for landscape A4 and warns of a view whose names would print
smaller than 6 pt once it is shrunk to fit. ortho already sets out what it can
in rows: use cases, packages that import nothing from each other, and packages
that all import the same one. What it cannot shorten is a view that simply
holds too much. Then split the view, since more views, each about one thing,
read better than one crowded one:

- **Logical view:** expose one branch of a big tree, `expose MakeBeverage::prepareWater;`,
  in a view of its own, and keep the whole tree's view to its upper levels by
  exposing a smaller tree. A tree of more than about twenty functions, or more
  than seven side by side under one parent, rarely fits.
- **Use case view:** one view for each actor, or each kind of use case,
  exposing a package of its own.
- **Implementation view:** expose one package, `expose Software::Onboard;`, in
  a view for each layer or stack, rather than the whole software at once.
- **Physical view:** expose one subsystem's `part`, not the whole product.
- **Process view:** one scenario per view; a scenario with more than about
  eight lifelines is two scenarios.
- **Deployment view:** one view for each host, or each kind of host.

## Use case view

What people ask the system for, and who asks (§7.25).

```sysml
package SystemUseCases {
    part def CoffeeMachine;
    part def Operator;
    part def Technician;

    use case def OperateMachine {
        subject machine : CoffeeMachine;
        actor operator : Operator;
    }

    use case makeEspresso : OperateMachine;

    use case makeCappuccino : OperateMachine {
        include makeEspresso;
    }

    use case descale : OperateMachine {
        subject machine;
        actor technician : Technician;
    }
}
```

| You write | The view draws |
|---|---|
| `use case name : Def;` | a «use case» box, `name : Def`. A `use case def` is drawn itself, as a «use case def» box, only when no usage is typed by it. |
| `subject name : System;` in a use case (def) | a «subject» box above each use case it is the subject of, joined to it by a line |
| `actor name : Type;` | a stick figure, `name : Type`, beside each use case it takes part in: primary ones, on a `use case def`, to its left, and secondary ones, added on a single use case, to its right |
| `include name;` | a solid «include» arrow down to that use case, below it. The target is a use case usage, never a definition. |
| `include use case name : Def;` | the same arrow, to a «use case» box of its own. This declares a use case rather than naming one, so next to a `use case name;` it is a second one. |

The subject is a use case's first parameter, so a use case that declares an
actor states its `subject` first, as `descale` does. Restated, it keeps its
type: `descale`'s `machine` is the same `machine : CoffeeMachine`. Type the
subject by a part def of its own, the system as a black box, as OMG's models
do.

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
| `action def Name { ... }` in a package | the root of a tree, an «action def» box |
| `action name;` or `action name { ... }` nested in another | an «action» box under its parent, which owns it: a filled diamond at the parent's end |
| `action name : Def;` in a package | nothing. It is the usage a `perform` chain starts from. |
| `expose Tree::branch;` in the view, naming a nested `action` | that branch alone, the function at its root: a tree too big for one page, split |
| `#refinement dependency Tree to SystemUseCases::Name;` in a package | a dashed arrow from the tree's root up to the use case, when the view exposes both |
| `expose SystemUseCases::Name;` in the view | the use case, above the trees that refine it. One nothing refines stands alone, a visible gap. |

**Refining a use case** (§7.3). A function tree says which use case it details
with a dependency, tagged from the standard library's `ModelingMetadata`, whose
source is "a more precise and/or accurate representation" than its target. It
is optional, but its ends are checked: once it names a use case, that use case
has to be loaded. A logical view that shows use cases is there to tie its trees
to them, though, so ortho warns of a tree it draws that refines no use case,
and renders anyway. For a view of a single branch, the tree the branch belongs
to has to refine one.

```sysml
package SystemUseCases {
    use case def OperateMachine;
}

package Functions {
    private import StandardViewDefinitions::*;
    private import ModelingMetadata::*;

    action def MakeBeverage {
        action heatWater;
    }

    #refinement dependency MakeBeverage to SystemUseCases::OperateMachine;

    view 'Logical view' : GeneralView {
        expose MakeBeverage;
        expose SystemUseCases::OperateMachine;
    }
}
```

## Implementation view

The software, as packages of modules (§7.5, §7.11).

```sysml
package Software {
    private import Functions::*;

    package Control {
        part def BrewController {
            perform beverage.controlMachine.interlockBrewGroup;
        }
    }

    package Application {
        private import Software::Control::*;

        part def RecipeEngine {
            perform beverage.controlMachine.sequenceRecipe;
        }
    }

    part sw {
        part recipeEngine : Application::RecipeEngine;
        part brewController : Control::BrewController;
    }
}

package Functions {
    action def MakeBeverage {
        action controlMachine {
            action sequenceRecipe;
            action interlockBrewGroup;
        }
    }
    action beverage : MakeBeverage;
}
```

| You write | The view draws |
|---|---|
| `package Name { ... }` | a package box, nested inside its parent package |
| `part def Name;` in a package | a module, a «part def» box, inside its package |
| `part def Name :> Base;` | the same, headed `Name :> Base` |
| `private import Pkg::*;` (or `Pkg::Element`, `Pkg::**`) | a dashed arrow to `Pkg`: «import»* (or «import», «import»**) |
| `part sw { part x : Module; ... }` in a package | nothing. The deployment view allocates from it. |
| `perform Functions::use.branch.leaf;` in a module | a line in its *perform actions* compartment |
| `port api : SomeApi;` in a module | a line in its *ports* compartment: an API it offers |
| `port api : ~SomeApi;` in a module | the same, conjugated: an API it needs |
| `port def SomeApi { ... }` | nothing. It types ports. |
| a package holding no part, such as the function tree | nothing |

**APIs** (§7.12). An API is a port, as on any part. The port definition's
features say what crosses with `in`, `out` or `inout`, and the module that
needs the API declares the port conjugated with `~`, which reverses every
direction (§7.12.3). Where modules publish and subscribe, the port definition
is the topic.

```sysml
package Software {
    private import ScalarValues::*;

    port def BrewApi {
        in attribute dose_g : Real;
        out attribute shotPoured : Boolean;
        inout attribute profile : String;
    }

    part def BrewController {
        port brew : BrewApi;
    }

    part def RecipeEngine {
        port brew : ~BrewApi;
    }
}
```

**On its own.** `perform` is optional, so an implementation view can stand
alone, as for a cloud solution built from a vendor's services. Declare the
services in a package of their own: ortho knows the names in OMG's library, but
no vendor's. To show which service talks to which, use them as parts in an
`InterconnectionView`, as on the physical view.

```sysml
package Aws {
    part def Lambda;
    part def DynamoDB;
}

package OrderSolution {
    private import StandardViewDefinitions::*;
    private import Aws::*;

    part def OrderHandler :> Lambda;
    part def OrderTable :> DynamoDB;

    view 'Order solution' : GeneralView {
        expose OrderSolution;
        expose Aws;
    }
}
```

## Physical view

The product: parts nested in the parts that contain them, wired through their
ports (§7.11–§7.14).

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
        interface : ControlLink connect board.pumpCtrl to pumps.ctrlIn;
    }

    part machine : Machine;
}
```

| You write | The view draws |
|---|---|
| `part name : Def [n];` | a «part» box, `name : Def [n]`, with the parts of `Def` nested inside |
| the `part` the view exposes | the top-level box |
| `port name : PortDef;` | a port on the box's edge. `~PortDef` conjugates it. |
| `connect a.p to b.q;` | a line between the two ports (or parts) |
| `interface : Def connect a.p to b.q;` (optionally named), or a `connection` of a `connection def` | the same line, labelled as declared: `: Def` |
| `interface def` with its `end`s, `port def` | nothing. They type interfaces and ports. |
| `attribute name : Type = value;` | nothing |
| `perform Functions::use.branch.leaf;` in a part or its definition | a line in its *perform actions* compartment |

What a part inherits through `:>`, or performs through its definition, is
drawn as if declared.

## Deployment view

Which computer runs which software (§7.15). The file declares nothing of its
own: it maps software usages onto hardware usages.

```sysml
package Software {
    part def RecipeEngine;
    part def UiApp;
    part sw {
        part recipeEngine : RecipeEngine;
        part uiApp : UiApp;
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
    private import Software::*;
    private import Hardware::*;

    allocate sw.recipeEngine to machine.controller;
    allocation uiOnPanel allocate sw.uiApp to machine.display;
}
```

| You write | The view draws |
|---|---|
| `allocate sw.part to host;` | a «part» box for the host, listing `allocated sw::part` in its *relationships* compartment, which takes a qualified name (§8.2.3.5) |
| `allocation name allocate ... to ...;` | the same; the name is not drawn |

Software allocated to two hosts is listed on both. Name each software usage
after its module, `part recipeEngine : RecipeEngine`: this view lists it by name
alone, and the implementation view draws the module. Two usages of one module
need names of their own.

The view adds nothing of its own: it maps what the other views draw onto each
other. Once a model declares an implementation view, ortho warns of software
that is not a usage of a module that view draws; once it declares a physical
view, of a host that is not a part that view draws. A port stands for the part
it is on.

## Process view

One scenario: who sends what to whom, in order (§7.9, §7.16).

```sysml
package Scenario {
    part def RecipeEngine;
    part def BrewController;

    item def DoseSetting;

    occurrence def Brew {
        part user;
        part recipeEngine : RecipeEngine;
        part brewController : BrewController;

        message order from user to recipeEngine;
        then message grind of DoseSetting from recipeEngine to brewController;
        then message extract from brewController to brewController;
        then message finished from brewController to recipeEngine;
    }
}
```

| You write | The view draws |
|---|---|
| `part name : Def;`, or a person's untyped `part name;`, in the scenario's `occurrence def` | a lifeline headed by a «part» box, once a message names it, in order of first appearance |
| `message name from a to b;` | an arrow from `a` to `b`, labelled `name`, in document order. The ends are parts, or `event occurrence`s on them. |
| `message name of Payload from a to b;` | the same arrow, labelled `name of Payload` |
| `item def Payload;` | nothing. It types a payload. |
| `then` before a message | nothing. Document order already gives the order. |

## Realization

A component names the functions it realizes, in its own body, with `perform`
(§7.17.6): the function is "defined in a different context than that of the
performer". ortho uses `perform`, not `allocate`, because the logical view
holds functions rather than components; `allocate` joins components, on the
deployment view.

```sysml
package Functions {
    action def MakeBeverage {
        action prepareWater { action heatWater; }
    }
    action beverage : MakeBeverage;
}

package Hardware {
    private import Functions::*;

    part def Thermoblock {
        perform beverage.prepareWater.heatWater;
    }
}
```

It is many to many: a program and a device may realize one function together,
and a part may realize several.

Once a model declares a logical view and an implementation or physical view,
every function has to have a solution. ortho warns of each function at the
bottom of a tree a logical view draws that no module of the implementation
view, and no part of the physical view, performs, either itself or through a
function above it, and renders anyway. A `perform` in a part that no view
draws does not count. Performing a typed action, `perform beverage;`, performs
its definition's whole tree. In a model with a use case view, only a tree that
refines a use case needs a solution, so a catalogue of functions can sit beside
the product's until a use case asks for one. A model with no implementation or
physical view gets no warning, since `perform` is optional.

A `perform` may also name a use case, listed the same way. One that names
structure, such as a part or a port, is an error.

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

## Parsed, not drawn

Every other definition and usage, from items and requirements to states,
calculations and metadata, parses, but no view draws it, and what it names is
kept as written rather than checked.

**Requirements** (§7.21). A non-functional requirement often proposes a
solution on the implementation or physical view, so a model carrying
requirements has to keep loading. `satisfy` names a requirement and the part
that satisfies it.

```sysml
package DroneRequirements {
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

**Values and expressions** (KerML §8.2.5.8): `capacity_mAh : Real = 10000`
above, a quantity with its unit, `mass = 1500 [kg]`, a calculation, or a
constraint's condition. No view evaluates them.

## The standard library

ortho does not load the standard library, so importing one of its packages,
such as `ScalarValues`, imports nothing. It knows the library's names, though: a
view may name a library element, such as a part typed
`SpatialItems::SpatialItem`, and shows it as written. A name the library does
not have is an error, as anywhere else.

## How this is checked

Tests keep this page honest: every `sysml` block above parses on its own, with
no errors or warnings. Another loads all 252 of OMG's own example, training and
validation models.

Both example models and every block on this page also passed the OMG pilot
implementation, release 0.63.1, on 2026-10-31, with no errors or warnings.
That is a one-off check, not part of CI; [DEVELOPMENT.md](DEVELOPMENT.md)
describes how to repeat it when the grammar grows.
