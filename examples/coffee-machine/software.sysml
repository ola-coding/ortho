// Software domain: the component architecture and its deployment onto the
// electronic compute nodes.
//
// Owns the compute budget — CPU load, memory and latency on each node it is
// allocated to. The allocations below are the deployment: every software part
// lands on a part of the machine defined in the electronic domain.
package Software {

    import System::*;
    import Requirements::*;

    // --- architecture -----------------------------------------------------
    part def RecipeEngine {
        attribute beverages : String = "coffee_espresso_cappuccino";
    }
    part def BrewController {
        attribute doseRange_g : String = "6_to_16";
    }
    part def TemperatureController {
        attribute controlLoop : String = "pid";
    }
    part def MilkController {
        attribute frothProfiles : String = "foam_and_hot_milk";
    }
    part def UiApp {
        attribute ui : String = "touch_beverage_selection";
    }

    part sw {
        part recipes : RecipeEngine;
        part brew : BrewController;
        part temperature : TemperatureController;
        part milk : MilkController;
        part uiApp : UiApp;
    }

    // --- deployment onto the electronic domain ----------------------------
    allocate sw.recipes to System::CoffeeMachine::controller;
    allocate sw.brew to System::CoffeeMachine::controller;
    allocate sw.temperature to System::CoffeeMachine::controller;
    allocate sw.milk to System::CoffeeMachine::controller;
    allocate sw.uiApp to System::CoffeeMachine::display;

    // --- requirement traces owned by this domain --------------------------
    satisfy Requirements::oneTouchReq by RecipeEngine;
}
