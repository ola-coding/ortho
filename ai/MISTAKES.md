# Mistakes to avoid

Mistakes agents make in SysML v2 models that ortho draws. Each one shows the
wrong shape, what ortho prints for it, and the right shape. Where ortho prints
nothing, the model loads, but it says something other than what was meant:
look for these yourself. Section numbers (§) point into OMG SysML v2.0,
Part 1, as on [LANGUAGE.md](../LANGUAGE.md).

A test checks this page: every wrong shape draws the message shown, or none
where it says so, and every right shape loads with no error or warning.

## SysML v1 habits

### A block

SysML v2 has no blocks or stereotypes (§7.6). A kind of part is a
`part def`, and a usage of it a `part`.

```sysml wrong
package Hardware {
    block def Engine;
}
```

ortho prints: `Expected ';', '{' or '}' but found 'def'.`

```sysml
package Hardware {
    part def Engine;
}
```

### A stereotype in a comment

A `«subsystem»` written in a `doc` is text, not a kind: no tool can find
the subsystems by it. Say what the element is with its own keyword and its
definition, and use the `doc` to describe it.

```sysml wrong
package Hardware {
    part def Engine {
        doc /* «subsystem» */
    }
}
```

ortho prints nothing.

```sysml
package Hardware {
    part def Subsystem;
    part def Engine :> Subsystem {
        doc /* Turns fuel into torque for the drivetrain. */
    }
}
```

## Names and imports

### A reserved word as a name

SysML v2 reserves 128 words, even those ortho does not draw, such as
`frame`, `state`, `item`, `flow` and `out` (§8.2.2.1.2).

```sysml wrong
package Hardware {
    part def Frame;
    part def Drone {
        part frame : Frame;
    }
}
```

ortho prints: `Expected ';' or '{' but found 'frame'.`

```sysml
package Hardware {
    part def Frame;
    part def Drone {
        part airframe : Frame;
    }
}
```

### An import with no visibility

An import states its visibility (§7.5.3), and `private` is almost always
the right one.

```sysml wrong
package Hardware {
    import ScalarValues::*;
    part def Tank {
        attribute full : Boolean;
    }
}
```

ortho prints: `Expected a declaration or an expression but found 'import'.`

```sysml
package Hardware {
    private import ScalarValues::*;
    part def Tank {
        attribute full : Boolean;
    }
}
```

### A package named after the standard library's

A top-level package named `Requirements`, `UseCases` or `Views` hides the
library's own, and other tools may find the library's instead.

```sysml wrong
package Requirements {
    part def Drone;
}
```

ortho prints: `Package 'Requirements' has the name of a standard library package and hides it, and other tools may find the library's instead. Rename it.`

```sysml
package DroneRequirements {
    part def Drone;
}
```

### A unit in a name

A unit in a name says to a reader what a value with its unit says to a tool.
Give the quantity its type and the value its unit.

```sysml wrong
package Hardware {
    private import ScalarValues::*;

    part def Tank {
        attribute capacity_l : Real = 1.9;
    }
}
```

ortho prints nothing.

```sysml
package Hardware {
    private import ISQ::*;
    private import SI::*;

    part def Tank {
        attribute capacity : VolumeValue = 1.9 [L];
    }
}
```

## Typing and references

### A part typed by an action

A part is typed by a part definition (§8.3). What a part does is a function
it performs, not its type.

```sysml wrong
package Machine {
    action def Brew;

    part def CoffeeMachine {
        part controller : Brew;
    }
}
```

ortho prints: `'Brew' is an action definition. A part is typed by a part definition.`

```sysml
package Machine {
    action def Brew;
    action brew : Brew;

    part def Board {
        perform brew;
    }

    part def CoffeeMachine {
        part controller : Board;
    }
}
```

### A chain through a definition

A feature chain starts at a usage and continues with dots (§7.6.6), so the
file that holds a tree declares one usage of it for others to start from.

```sysml wrong
package Functions {
    action def MakeBeverage {
        action prepareWater {
            action heatWater;
        }
    }
}

package Hardware {
    part def Thermoblock {
        perform Functions::MakeBeverage::prepareWater.heatWater;
    }
}
```

ortho prints: `'Functions::MakeBeverage::prepareWater' names a feature through its owner. Start at a usage and continue with dots.`

```sysml
package Functions {
    action def MakeBeverage {
        action prepareWater {
            action heatWater;
        }
    }
    action beverage : MakeBeverage;
}

package Hardware {
    part def Thermoblock {
        perform Functions::beverage.prepareWater.heatWater;
    }
}
```

### Software allocated by its definition

`allocate` joins usages (§7.15): one usage of each module, in the tree of
deployable software, to the part that runs it.

```sysml wrong
package Application {
    part def RecipeEngine;
}

package Hardware {
    part def Board;
    part def Machine {
        part controller : Board;
    }
    part machine : Machine;
}

package Deployment {
    private import Application::*;
    private import Hardware::*;

    allocate RecipeEngine to machine.controller;
}
```

ortho prints: `Could not resolve reference to Feature named 'RecipeEngine'.`

```sysml
package Application {
    part def RecipeEngine;
}

package Hardware {
    part def Board;
    part def Machine {
        part controller : Board;
    }
    part machine : Machine;
}

package Deployment {
    private import Application::*;
    private import Hardware::*;

    part sw {
        part recipeEngine : RecipeEngine;
    }

    allocate sw.recipeEngine to machine.controller;
}
```

## Use cases

### Including a use case definition

`include` names a use case usage, never a definition (§7.25).

```sysml wrong
package SystemUseCases {
    part def CoffeeMachine;
    part def Operator;

    use case def OperateMachine {
        subject machine : CoffeeMachine;
        actor operator : Operator;
    }

    use case def MakeEspresso;

    use case makeCappuccino : OperateMachine {
        include MakeEspresso;
    }
}
```

ortho prints: `Could not resolve reference to Feature named 'MakeEspresso'.`

```sysml
package SystemUseCases {
    part def CoffeeMachine;
    part def Operator;

    use case def OperateMachine {
        subject machine : CoffeeMachine;
        actor operator : Operator;
    }

    use case makeEspresso : OperateMachine;

    use case makeCappuccino : OperateMachine {
        include makeEspresso;
    }
}
```

### An actor before the subject

The subject is a use case's first parameter, so a use case that adds an
actor restates its subject first (§7.25).

```sysml wrong
package SystemUseCases {
    part def CoffeeMachine;
    part def Operator;
    part def Technician;

    use case def OperateMachine {
        subject machine : CoffeeMachine;
        actor operator : Operator;
    }

    use case descale : OperateMachine {
        actor technician : Technician;
        subject machine;
    }
}
```

ortho prints: `Declare the subject of 'descale' before its actors: the subject is a use case's first parameter.`

```sysml
package SystemUseCases {
    part def CoffeeMachine;
    part def Operator;
    part def Technician;

    use case def OperateMachine {
        subject machine : CoffeeMachine;
        actor operator : Operator;
    }

    use case descale : OperateMachine {
        subject machine;
        actor technician : Technician;
    }
}
```

## Functions and their realization

### A function named after what performs it

The logical view holds what the system does, whatever ends up doing it
([VIEWS.md](../VIEWS.md)). A function named after a part or a program says
how, and is wrong once the design changes. Name it with a verb and what it
acts on.

```sysml wrong
package Functions {
    action def MakeBeverage {
        action pumpController;
        action thermoblock;
    }
}
```

ortho prints nothing.

```sysml
package Functions {
    action def MakeBeverage {
        action pumpWater;
        action heatWater;
    }
}
```

### A function allocated instead of performed

A part or a module names the functions it realizes with `perform`, in its
own body (§7.17.6). `allocate` maps software onto hardware, on the
deployment view, and does not realize a function.

```sysml wrong
package Functions {
    private import StandardViewDefinitions::*;

    action def MakeBeverage {
        action heatWater;
    }
    action beverage : MakeBeverage;

    view 'Logical view' : GeneralView {
        expose MakeBeverage;
    }
}

package Hardware {
    private import StandardViewDefinitions::*;

    part def Thermoblock;
    part def Machine {
        part thermoblock : Thermoblock;
    }
    part machine : Machine;

    view 'Physical view' : InterconnectionView {
        expose machine;
    }
}

package Deployment {
    private import Functions::*;
    private import Hardware::*;

    allocate beverage.heatWater to machine.thermoblock;
}
```

ortho prints: `No part or module of the implementation or physical view performs 'heatWater'. Perform it, or a function above it, in the one that realizes it.`

```sysml
package Functions {
    private import StandardViewDefinitions::*;

    action def MakeBeverage {
        action heatWater;
    }
    action beverage : MakeBeverage;

    view 'Logical view' : GeneralView {
        expose MakeBeverage;
    }
}

package Hardware {
    private import StandardViewDefinitions::*;
    private import Functions::*;

    part def Thermoblock {
        perform beverage.heatWater;
    }
    part def Machine {
        part thermoblock : Thermoblock;
    }
    part machine : Machine;

    view 'Physical view' : InterconnectionView {
        expose machine;
    }
}
```

## Views

### One package around all the software

Everything the implementation view draws is software, so a `Software`
package around the layers says nothing: it only draws a frame around the
rest. Make each layer or stack a package of its own, and expose each.

```sysml wrong
package Software {
    package Application {
        part def RecipeEngine;
    }
    package Control {
        part def BrewController;
    }
}

view 'Implementation view' : StandardViewDefinitions::GeneralView {
    expose Software;
}
```

ortho prints nothing.

```sysml
package Application {
    part def RecipeEngine;
}

package Control {
    part def BrewController;
}

view 'Implementation view' : StandardViewDefinitions::GeneralView {
    expose Application;
    expose Control;
}
```
