// Deployment view: which board runs which software.
package Deployment {
    doc /* Declares nothing of its own: its whole content is the mapping between
         * the parts in implementation.sysml and the parts in physical.sysml, so
         * it is rendered together with both. */

    private import Software::*;
    private import Hardware::*;
    private import StandardViewDefinitions::*;

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
