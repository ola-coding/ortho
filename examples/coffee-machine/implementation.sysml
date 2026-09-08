// Implementation view: the software that realizes the control functions.
//
// Two layers: the application decides what beverage to make, the control layer
// drives one device each. The dependency runs one way, application → control.
// Nothing here says which board the software runs on; that is deployment.
package Software {

    package Control {
        part def BrewController;
        part def TemperatureController;
        part def MilkController;
    }

    package Application {
        import Software::Control;

        part def RecipeEngine;
        part def UiApp;
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
