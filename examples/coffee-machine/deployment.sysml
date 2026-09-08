// Deployment view: which board runs which software.
//
// This file declares nothing of its own. Its whole content is the mapping
// between the parts in implementation.sysml and the parts in physical.sysml,
// which is why it must be rendered together with both.
package Deployment {

    import Software::*;
    import Hardware::*;

    // One dual-core board carries all four control programs.
    allocate Software::sw.recipes to Hardware::CoffeeMachine::controller;
    allocate Software::sw.brew to Hardware::CoffeeMachine::controller;
    allocate Software::sw.temperature to Hardware::CoffeeMachine::controller;
    allocate Software::sw.milk to Hardware::CoffeeMachine::controller;

    // The panel runs its own UI.
    allocate Software::sw.uiApp to Hardware::CoffeeMachine::display;
}
