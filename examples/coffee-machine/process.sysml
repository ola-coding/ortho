// Process view: one scenario — the software making a cappuccino.
//
// The lifelines are the software modules from implementation.sysml, so that
// file is rendered alongside this one.
package MakeCappuccinoScenario {

    private import Software::*;
    private import StandardViewDefinitions::*;

    // What the messages carry. Values, not things, so attribute defs.
    attribute def BeverageRecipe;
    attribute def TempSetpoint;
    attribute def DoseSetting;
    attribute def MilkSetting;

    action def MakeCappuccino {
        part user;
        part ui : Application::UiApp;
        part recipes : Application::RecipeEngine;
        part temperature : Control::TemperatureController;
        part brew : Control::BrewController;
        part milk : Control::MilkController;

        message selectCappuccino from user to ui;
        then message startBeverage of BeverageRecipe from ui to recipes;
        then message heatTo of TempSetpoint from recipes to temperature;
        then message atTemperature from temperature to recipes;
        then message grindAndBrew of DoseSetting from recipes to brew;
        then message extractShot from brew to brew;
        then message shotComplete from brew to recipes;
        then message frothMilk of MilkSetting from recipes to milk;
        then message beverageReady from recipes to ui;
    }

    view 'Process view — Coffee Machine' : SequenceView {
        expose MakeCappuccino;
    }
}
