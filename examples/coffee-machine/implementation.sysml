// Implementation view: the software that realizes the control functions.
//
// Two layers: the application decides what beverage to make, the control layer
// drives one device each. The dependency runs one way, application → control.
// Nothing here says which board the software runs on; that is deployment.
package Software {

    // The functions this software realizes, said on the module that realizes
    // each one. Which board runs it is deployment's business.
    private import Functions::*;
    private import ScalarValues::*;

    // The APIs the modules offer each other. A module that offers one declares
    // a port of that type; a module that needs it declares the same type
    // conjugated, `~RecipeApi`, so the two ends mirror each other.
    port def RecipeApi {
        in attribute beverage : String;
        out attribute progress : String;
    }

    port def BrewApi {
        in attribute dose_g : Real;
        out attribute shotPoured : Boolean;
    }

    port def TemperatureApi {
        in attribute setpoint_C : Real;
        out attribute atSetpoint : Boolean;
    }

    port def MilkApi {
        in attribute frothSeconds : Real;
        out attribute frothed : Boolean;
    }

    package Control {
        part def BrewController {
            perform Functions::beverage.prepareCoffee.doseGrounds;
            perform Functions::beverage.controlMachine.interlockBrewGroup;
            // It owns the brew group, so it owns flushing it out.
            perform Functions::maintenance.runDescaleCycle;

            port brew : BrewApi;
        }

        part def TemperatureController {
            perform Functions::beverage.controlMachine.regulateTemperature;

            port temperature : TemperatureApi;
        }

        part def MilkController {
            perform Functions::beverage.prepareMilk.frothMilk;

            port milk : MilkApi;
        }
    }

    package Application {
        private import Software::Control;

        // It offers one API and drives three: the whole of the machine's
        // control is this module calling those.
        part def RecipeEngine {
            perform Functions::beverage.controlMachine.sequenceRecipe;

            port recipe : RecipeApi;
            port brew : ~BrewApi;
            port temperature : ~TemperatureApi;
            port milk : ~MilkApi;
        }

        part def UiApp {
            perform Functions::beverage.controlMachine.selectBeverage;
            perform Functions::maintenance.signalDescaleDue;

            port recipe : ~RecipeApi;
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
