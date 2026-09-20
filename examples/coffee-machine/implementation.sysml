// Implementation view: the software that realizes the control functions.
//
// Two layers: the application decides what beverage to make, the control layer
// drives one device each. The dependency runs one way, application → control.
// Nothing here says which board the software runs on; that is deployment.
package Software {

    // The functions this software realizes, said on the module that realizes
    // each one. Which board runs it is deployment's business.
    private import Functions::*;

    package Control {
        part def BrewController {
            perform Functions::beverage.prepareCoffee.doseGrounds;
            perform Functions::beverage.controlMachine.interlockBrewGroup;
            // It owns the brew group, so it owns flushing it out.
            perform Functions::maintenance.runDescaleCycle;
        }

        part def TemperatureController {
            perform Functions::beverage.controlMachine.regulateTemperature;
        }

        part def MilkController {
            perform Functions::beverage.prepareMilk.frothMilk;
        }
    }

    package Application {
        private import Software::Control;

        part def RecipeEngine {
            perform Functions::beverage.controlMachine.sequenceRecipe;
        }

        part def UiApp {
            perform Functions::beverage.controlMachine.selectBeverage;
            perform Functions::maintenance.signalDescaleDue;
        }
    }

    // The deployable parts, named once so deployment.sysml can allocate them.
    part sw {
        part recipes : Application::RecipeEngine;
        part uiApp : Application::UiApp;
        part brew : Control::BrewController;
        part temperature : Control::TemperatureController;
        part milk : Control::MilkController;
    }
}
