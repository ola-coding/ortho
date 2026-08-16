// Process view scenario: one press makes a cappuccino.
// Select the beverage, the machine heats, grinds and extracts a shot,
// froths milk on top, then reports it is ready.
package MakeCappuccinoScenario {

    import Software::*;

    action def MakeCappuccino {
        part user;
        part ui : Software::UiApp;
        part recipes : Software::RecipeEngine;
        part temperature : Software::TemperatureController;
        part brew : Software::BrewController;
        part milk : Software::MilkController;

        message selectCappuccino from user to ui;
        then message startBeverage of BeverageRecipe from ui to recipes;
        then message heatTo of TempSetpoint from recipes to temperature;
        then message grindAndBrew of DoseSetting from recipes to brew;
        then message extractShot from brew to brew;
        then message shotComplete from brew to recipes;
        then message frothMilk of MilkSetting from recipes to milk;
        then message beverageReady from recipes to ui;
    }
}
