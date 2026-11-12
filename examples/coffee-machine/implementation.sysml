// Implementation view: the software that realizes the control functions.
package Software {
    doc /* Two layers: the application decides what beverage to make, the control
         * layer drives one device each. The dependency runs one way, application
         * to control. */

    // Which board runs what is deployment.sysml's business, not this file's.
    private import Functions::*;
    private import ScalarValues::*;
    private import ISQ::*;
    private import StandardViewDefinitions::*;

    // What crosses the APIs. process.sysml sends the same items.
    item def BeverageRecipe {
        attribute beverage : String;
    }
    item def TempSetpoint {
        attribute temperature : CelsiusTemperatureValue;
    }
    item def DoseSetting {
        attribute dose : MassValue;
    }
    item def MilkSetting {
        attribute frothTime : DurationValue;
    }

    // The APIs the modules offer each other. A module that offers one declares
    // a port of that type; a module that needs it declares the same type
    // conjugated, `~RecipeApi`, so the two ends mirror each other.
    port def RecipeApi {
        in item recipe : BeverageRecipe;
        out attribute ready : Boolean;
    }

    port def TemperatureApi {
        in item setpoint : TempSetpoint;
        out attribute atSetpoint : Boolean;
    }

    port def BrewApi {
        in item dose : DoseSetting;
        out attribute shotPoured : Boolean;
    }

    port def MilkApi {
        in item milk : MilkSetting;
        out attribute frothed : Boolean;
    }

    package Application {
        private import Software::Control;

        part def UiApp {
            perform beverage.controlMachine.selectBeverage;

            port recipe : ~RecipeApi;
        }

        part def RecipeEngine {
            doc /* Offers one API and drives three: the whole of the machine's
                 * control is this module calling those. */
            perform beverage.controlMachine.sequenceRecipe;

            port recipe : RecipeApi;
            port temperature : ~TemperatureApi;
            port brew : ~BrewApi;
            port milk : ~MilkApi;
        }
    }

    package Control {
        part def TemperatureController {
            perform beverage.controlMachine.regulateTemperature;

            port temperature : TemperatureApi;
        }

        part def BrewController {
            doc /* Doses, tamps and extracts, and keeps the brew group shut while
                 * it is under pressure. */
            perform beverage.prepareCoffee.extractShot;

            port brew : BrewApi;
        }

        part def MilkController {
            perform beverage.prepareMilk.frothMilk;

            port milk : MilkApi;
        }
    }

    // The deployable parts, in the order of the modules above, so
    // deployment.sysml can allocate them.
    part sw {
        part uiApp : Application::UiApp;
        part recipeEngine : Application::RecipeEngine;
        part temperatureController : Control::TemperatureController;
        part brewController : Control::BrewController;
        part milkController : Control::MilkController;
    }

    // The view shows the two layers themselves, without Software around them.
    view 'Implementation view — Coffee Machine' : GeneralView {
        expose Application;
        expose Control;
    }
}
