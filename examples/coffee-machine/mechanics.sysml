// Mechanical domain: the design that realizes the machine's fluid, thermal and
// grinding blocks.
//
// Owns the physical budget — mass, geometry, pressure, flow and thermal. The
// subsystem definitions here specialize the architecture blocks declared in
// System, so this package depends on the system level rather than the other way
// round, and the components below are private to the domain.
package Mechanics {

    import System::*;
    import Requirements::*;

    // Dry coffee, whole beans or ground. Never leaves this domain, so it is
    // declared here rather than among the system's cross-domain interfaces.
    port def CoffeePort;

    // --- water path components --------------------------------------------
    part def WaterTank {
        attribute capacity_l : Real = 1.9;
        port waterOut : System::WaterPort;
    }

    part def WaterPump {
        attribute maxPressure_bar : Real = 15;
        port waterIn : System::WaterPort;
        port waterOut : System::WaterPort;
        port ctrlIn : System::DataPort;
    }

    part def Thermoblock {
        attribute targetTemp_C : Real = 92;
        port waterIn : System::WaterPort;
        port hotWaterOut : System::WaterPort;
        port steamOut : System::SteamPort;
        port ctrlIn : System::DataPort;
    }

    // Realization: a pumped thermoblock circuit. Ports come from the block it
    // specializes; only the internals and their wiring are declared here.
    part def ThermoblockWaterSubsystem :> System::WaterSubsystem {
        part tank : WaterTank;
        part pump : WaterPump;
        part heater : Thermoblock;

        connect tank.waterOut to pump.waterIn;
        connect pump.waterOut to heater.waterIn;
        connect heater.hotWaterOut to hotWaterOut;
        connect heater.steamOut to steamOut;
        connect ctrlIn to pump.ctrlIn;
        connect ctrlIn to heater.ctrlIn;
    }

    // --- coffee path components -------------------------------------------
    part def BeanHopper {
        attribute capacity_g : Real = 280;
        port beansOut : CoffeePort;
    }

    part def Grinder {
        attribute burrs : String = "conical_ceramic";
        port beansIn : CoffeePort;
        port groundsOut : CoffeePort;
        port ctrlIn : System::DataPort;
    }

    part def BrewUnit {
        attribute chamberVolume_g : Real = 16;
        port waterIn : System::WaterPort;
        port groundsIn : CoffeePort;
        port coffeeOut : System::BeveragePort;
        port ctrlIn : System::DataPort;
    }

    // Realization: grind fresh per cup, straight into the brew chamber.
    part def BurrCoffeeSubsystem :> System::CoffeeSubsystem {
        part hopper : BeanHopper;
        part grinder : Grinder;
        part brewUnit : BrewUnit;

        connect hopper.beansOut to grinder.beansIn;
        connect grinder.groundsOut to brewUnit.groundsIn;
        connect waterIn to brewUnit.waterIn;
        connect brewUnit.coffeeOut to coffeeOut;
        connect ctrlIn to grinder.ctrlIn;
        connect ctrlIn to brewUnit.ctrlIn;
    }

    // --- milk --------------------------------------------------------------
    // Realization: steam wand fed from the water subsystem's steam port.
    part def SteamMilkFrother :> System::MilkFrother {
        attribute texture : String = "fine_foam";
    }

    // --- requirement traces owned by this domain --------------------------
    satisfy Requirements::pressureReq by WaterPump;
    satisfy Requirements::temperatureReq by Thermoblock;
    satisfy Requirements::waterReq by WaterTank;
    satisfy Requirements::beanReq by BeanHopper;
}
