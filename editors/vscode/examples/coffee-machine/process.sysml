// Process view: one scenario, the software making a cappuccino.
package MakeCappuccinoScenario {
    doc /* The user picks a cappuccino on the panel; the recipe engine heats,
         * brews and froths in turn. */

    private import Software::*;
    private import StandardViewDefinitions::*;

    // The lifelines are the user and the modules of implementation.sysml; each
    // message is an item one of their APIs carries.
    occurrence def MakeCappuccino {
        part user : SystemUseCases::User;
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
