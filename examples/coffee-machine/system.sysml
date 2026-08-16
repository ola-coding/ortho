// System level: the logical architecture of the machine.
//
// Two things live here and nothing else: the interfaces the domains exchange
// across, and the blocks that expose them. Each block declares only its ports —
// no internals, and no reference to any engineering domain. The domains
// specialize these blocks to supply the design that realizes them, so the
// dependency runs domain -> system and this package depends on nothing at all.
package System {

    // --- cross-domain interfaces ------------------------------------------
    // Only interfaces that cross a domain boundary belong here. A port used
    // inside one domain stays in that domain; see Mechanics::CoffeePort.

    // Electronics owns the electrical budget: voltage, current, power draw.
    port def PowerPort {
        attribute voltage_V : Real = 230;
    }

    // Electronics owns signalling; mechanics consumes it on every actuator.
    port def DataPort;

    // Mechanics owns the physical budget: pressure, flow, mass, geometry.
    port def WaterPort {
        attribute pressure_bar : Real;
    }
    port def SteamPort;
    port def BeveragePort;

    // --- architecture blocks ----------------------------------------------
    part def WaterSubsystem {
        port hotWaterOut : WaterPort;
        port steamOut : SteamPort;
        port ctrlIn : DataPort;
    }

    part def CoffeeSubsystem {
        port waterIn : WaterPort;
        port coffeeOut : BeveragePort;
        port ctrlIn : DataPort;
    }

    part def MilkFrother {
        port steamIn : SteamPort;
        port milkOut : BeveragePort;
        port ctrlIn : DataPort;
    }

    part def DisplayModule {
        port ctrlOut : DataPort;
    }

    part def ControlBoard {
        port powerIn : PowerPort;
        port uiIn : DataPort;
        // One control interface per mechanical subsystem.
        port waterCtrl : DataPort;
        port coffeeCtrl : DataPort;
        port milkCtrl : DataPort;
    }

    part def PowerSupply {
        port powerOut : PowerPort;
    }

    // --- the machine ------------------------------------------------------
    part def CoffeeMachine {
        attribute width_mm : Real = 280;
        port beverageOut : BeveragePort;

        part water : WaterSubsystem;
        part coffee : CoffeeSubsystem;
        part frother : MilkFrother;
        part display : DisplayModule;
        part controller : ControlBoard;
        part psu : PowerSupply;

        // Electronic -> mechanical: the cross-domain control interfaces.
        connect controller.waterCtrl to water.ctrlIn;
        connect controller.coffeeCtrl to coffee.ctrlIn;
        connect controller.milkCtrl to frother.ctrlIn;

        // Within electronics.
        connect psu.powerOut to controller.powerIn;
        connect display.ctrlOut to controller.uiIn;

        // Within mechanics: the beverage path.
        connect water.hotWaterOut to coffee.waterIn;
        connect water.steamOut to frother.steamIn;
        connect coffee.coffeeOut to beverageOut;
        connect frother.milkOut to beverageOut;
    }
}
