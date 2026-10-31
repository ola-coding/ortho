// Deployment view: which board runs which software.
//
// This file declares nothing of its own. Its whole content is the mapping
// between the parts in implementation.sysml and the parts in physical.sysml,
// which is why it must be rendered together with both.
package Deployment {

    private import Software::*;
    private import Hardware::*;
    private import StandardViewDefinitions::*;

    // One dual-core board carries all four control programs.
    allocate sw.recipeEngine to machine.controller;
    allocate sw.brewController to machine.controller;
    allocate sw.temperatureController to machine.controller;
    allocate sw.milkController to machine.controller;

    // The panel runs its own UI.
    allocate sw.uiApp to machine.display;

    view 'Deployment view — Coffee Machine' : GeneralView {
        expose Deployment;
    }
}
