// Deployment view: which board runs which software.
package Deployment {
    doc /* The mapping between the modules of implementation.sysml and the parts
         * of physical.sysml, so it is rendered together with both. */

    private import Application::*;
    private import Control::*;
    private import Hardware::*;
    private import StandardViewDefinitions::*;

    // The deployable software: one usage of each module, named after it, for
    // the allocations below to start from.
    part sw {
        part uiApp : UiApp;
        part recipeEngine : RecipeEngine;
        part temperatureController : TemperatureController;
        part brewController : BrewController;
        part milkController : MilkController;
    }

    // The panel runs its own UI.
    allocate sw.uiApp to machine.display;

    // One dual-core board carries all four control programs.
    allocate sw.recipeEngine to machine.controller;
    allocate sw.temperatureController to machine.controller;
    allocate sw.brewController to machine.controller;
    allocate sw.milkController to machine.controller;

    view 'Deployment view — Coffee Machine' : GeneralView {
        expose Deployment;
    }
}
