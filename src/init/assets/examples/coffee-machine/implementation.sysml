// Implementation view: the software that realizes the control functions.
//
// Each layer is a package of its own, and the APIs between them another.
// Which board runs what is deployment.sysml's business, not this file's.

package Apis {
    doc /* What the modules offer each other. A module that offers an API
         * declares a port of that type; a module that needs it declares the
         * same type conjugated, `~RecipeApi`, so the two ends mirror each other. */
    private import ScalarValues::*;
    private import ISQ::*;

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
}

package Application {
    doc /* Decides what beverage to make. Depends on the control layer, never
         * the other way round. */
    private import Functions::*;
    private import Control;

    part def UiApp {
        perform beverage.controlMachine.selectBeverage;

        port recipe : ~Apis::RecipeApi;
    }

    part def RecipeEngine {
        doc /* Offers one API and drives three: the whole of the machine's
             * control is this module calling those. */
        perform beverage.controlMachine.sequenceRecipe;

        port recipe : Apis::RecipeApi;
        port temperature : ~Apis::TemperatureApi;
        port brew : ~Apis::BrewApi;
        port milk : ~Apis::MilkApi;
    }
}

package Control {
    doc /* Drives one device each. */
    private import Functions::*;

    part def TemperatureController {
        perform beverage.controlMachine.regulateTemperature;

        port temperature : Apis::TemperatureApi;
    }

    part def BrewController {
        doc /* Doses, tamps and extracts, and keeps the brew group shut while it
             * is under pressure. */
        perform beverage.prepareCoffee.extractShot;

        port brew : Apis::BrewApi;
    }

    part def MilkController {
        perform beverage.prepareMilk.frothMilk;

        port milk : Apis::MilkApi;
    }
}

view 'Implementation view — Coffee Machine' : StandardViewDefinitions::GeneralView {
    expose Application;
    expose Control;
}
