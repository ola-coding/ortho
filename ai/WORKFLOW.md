The `.sysml` files in a folder are one SysML v2 model. ortho reads them together and draws, as SVG, every view the model declares. Three files say what to write:

- [LANGUAGE.md](../LANGUAGE.md): what ortho reads, view by view, and what each view draws from it. Read it before you change a model, and write only what it describes.
- [VIEWS.md](../VIEWS.md): what each view is for, and so what belongs on which.
- [MISTAKES.md](MISTAKES.md): mistakes agents make, each with what ortho prints and the right shape.

## Build a model in order

A model answers its questions in this order, one view each:

1. **Use case view:** what people ask the system for, and who asks.
2. **Logical view:** what the system does, as trees of functions, each refining the use case it details. A function names what is done, never the part or program that does it.
3. **Implementation view** and **physical view:** the software modules and the hardware parts, each naming the functions it realizes with `perform`.
4. **Deployment view:** which computer runs which software.
5. **Process view:** one scenario at a time, who sends what to whom.

Every view is optional, so start where the user's question is. Once two views exist, though, they have to agree, and ortho warns where they do not:

- every function at the bottom of a tree is performed by a module or a part;
- every use case the logical view shows is refined by a function tree;
- every program is allocated to a computer.

Two complete models to follow, a coffee machine and a survey drone, are in https://github.com/ola-coding/ortho/tree/main/examples.

## Check your work with ortho

After every change, render the model's folder and read everything ortho prints:

```sh
ortho render <model folder> -o <a temporary folder>
```

Write the diagrams into the model's own folder only when asked to update them.

- **Errors**, under `Failed to parse:`, are what SysML v2 forbids, or a name that resolves to nothing. ortho draws nothing until they are fixed.
- **Warnings**, each starting `Warning:`, are views of the model that disagree: a function no part or module performs, a use case no function tree refines, software that no deployment allocates to a host. The model draws, but it is not finished.
- **A view too big for its page** warns too: `Warning: '<view>' is too wide for landscape A4`, or too tall. It names no line, since no line is wrong. See below.
- **`Skipped`** names a view ortho does not draw, and why.

Each error and model warning names its file and line. Fix every error and every warning before you finish. If you leave a warning, say which one and why.

## When a view is too big for its page

A real model outgrows one page, and that is as it should be. Split the view into several, each about one concern or one level of the system's breakdown, as *Keeping a view on one page* in LANGUAGE.md shows: one branch of a function tree, one subsystem, one layer of software, one scenario.

Never merge, shorten, rename or delete model elements to make a view fit. The model is the truth, and a view is a window onto part of it.

## Fix the cause, not the line

Never delete or comment out the line a problem points at to make it go away. A `perform`, a `#refinement dependency` or an `allocate` that no longer resolves means a name changed: find the new name, which the error suggests in a chain such as `beverage.prepareCoffee.storeBeans`, and update the reference. A deleted line leaves a function with nothing to realize it, or a use case nothing details.

When you rename an element, update every reference to it in every file of the folder, then render again.

## Say what you know and what you assumed

- **Invent no figures.** When the user gave no value for an attribute, leave the value out, or give an assumed one and say so in the element's `doc`, such as `doc /* Mass assumed: no figure given. */`. Tell the user which values you assumed.
- **Claim only checks you ran.** Say the model renders with no errors or warnings only after ortho has said so.
- **Say what ortho does not draw.** Asked for something no view draws, such as requirements or states, write it as *Parsed, not drawn* in LANGUAGE.md describes, and tell the user that no diagram shows it. Never bend it into the shape of a view that draws.

## Reviewing a model

When asked to review a model rather than change it:

1. Render it, and report every error and warning with its file and line.
2. Look for what ortho cannot see: each mistake in MISTAKES.md for which ortho prints nothing; functions named after the parts or programs that perform them; use cases that say how rather than what; elements a reader needs explained that have no `doc`; views each about more than one thing.
3. Report what you found, each with its file and line and the fix you suggest. Change the model only when asked.
