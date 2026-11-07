// Process view: one scenario — the software making a cappuccino.
//
// The lifelines are the software modules from implementation.sysml, so that
// file is rendered alongside this one.
package MakeCappuccinoScenario {

    private import Software::*;
    private import StandardViewDefinitions::*;

    // What the messages carry.
    item def BeverageRecipe;
    item def TempSetpoint;
    item def DoseSetting;
    item def MilkSetting;

    occurrence def MakeCappuccino {
        part user;
        part uiApp : Application::UiApp;
        part recipeEngine : Application::RecipeEngine;
        part temperatureController : Control::TemperatureController;
        part brewController : Control::BrewController;
        part milkController : Control::MilkController;

        message selectCappuccino from user to uiApp;
        then message startBeverage of BeverageRecipe from uiApp to recipeEngine;
        then message heatTo of TempSetpoint from recipeEngine to temperatureController;
        then message atTemperature from temperatureController to recipeEngine;
        then message grindAndBrew of DoseSetting from recipeEngine to brewController;
        then message extractShot from brewController to brewController;
        then message shotComplete from brewController to recipeEngine;
        then message frothMilk of MilkSetting from recipeEngine to milkController;
        then message beverageReady from recipeEngine to uiApp;
    }

    view 'Process view — Coffee Machine' : SequenceView {
        expose MakeCappuccino;
    }
}
