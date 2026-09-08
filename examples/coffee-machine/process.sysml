// Process view: one scenario — the software making a cappuccino.
//
// One scenario per file, because the view renders every message it can see.
// The lifelines are the software modules from implementation.sysml, so that
// file is rendered alongside this one.
package MakeCappuccinoScenario {

    import Software::*;

    action def MakeCappuccino {
        part user;
        part ui : Software::Application::UiApp;
        part recipes : Software::Application::RecipeEngine;
        part temperature : Software::Control::TemperatureController;
        part brew : Software::Control::BrewController;
        part milk : Software::Control::MilkController;

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
}
