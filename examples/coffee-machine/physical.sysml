// Physical view: the product itself — mechanics, electronics and cabling.
//
// Topology only. Which part connects, feeds or wires to which; never where a
// part sits or how big it is. A cable is a connection, so the control looms
// below are `connect` statements typed by the interface they carry.
package Hardware {

    // Real and String are the spec's own value types, from its library.
    private import ScalarValues::*;

    // The functions this hardware realizes, said on the part that realizes
    // each one. Storing milk is not among them: the machine has no milk tank,
    // and the logical view keeps the function whether or not anything does it.
    private import Functions::*;

    port def WaterPort;
    port def SteamPort;
    port def CoffeePort;
    port def BeveragePort;
    port def PowerPort;
    port def DataPort;

    // The contract every control loom satisfies.
    interface def ControlLink {
        end controller : DataPort;
        end device : DataPort;
    }

    // --- water path --------------------------------------------------------
    part def WaterTank {
        perform Functions::beverage.prepareWater.storeWater;

        attribute capacity_l : Real = 1.9;
        port waterOut : WaterPort;
    }

    part def WaterPump {
        perform Functions::beverage.prepareWater.pumpWater;

        attribute maxPressure_bar : Real = 15;
        port waterIn : WaterPort;
        port waterOut : WaterPort;
        port ctrlIn : DataPort;
    }

    part def Thermoblock {
        perform Functions::beverage.prepareWater.heatWater;

        attribute targetTemp_C : Real = 92;
        port waterIn : WaterPort;
        port hotWaterOut : WaterPort;
        port steamOut : SteamPort;
        port ctrlIn : DataPort;
    }

    // --- coffee path -------------------------------------------------------
    part def BeanHopper {
        perform Functions::beverage.prepareCoffee.storeBeans;

        attribute capacity_g : Real = 280;
        port beansOut : CoffeePort;
    }

    part def Grinder {
        perform Functions::beverage.prepareCoffee.grindBeans;

        attribute burrs : String = "conical_ceramic";
        port beansIn : CoffeePort;
        port groundsOut : CoffeePort;
        port ctrlIn : DataPort;
    }

    part def BrewUnit {
        perform Functions::beverage.prepareCoffee.extractShot;
        // The coffee leaves the machine here.
        perform Functions::beverage.serveBeverage;

        attribute chamberVolume_g : Real = 16;
        port waterIn : WaterPort;
        port groundsIn : CoffeePort;
        port coffeeOut : BeveragePort;
        port ctrlIn : DataPort;
    }

    // --- milk --------------------------------------------------------------
    part def SteamWand {
        perform Functions::beverage.prepareMilk.frothMilk;
        // And the milk, here.
        perform Functions::beverage.serveBeverage;

        port steamIn : SteamPort;
        port milkOut : BeveragePort;
        port ctrlIn : DataPort;
    }

    // --- electronics -------------------------------------------------------
    part def ControlBoard {
        attribute mcuCores : Real = 2;
        port powerIn : PowerPort;
        port uiLink : DataPort;
        port waterCtrl : DataPort;
        port coffeeCtrl : DataPort;
        port milkCtrl : DataPort;
    }

    part def TouchDisplay {
        attribute diagonal_in : Real = 4.3;
        port ctrlOut : DataPort;
    }

    part def MainsSupply {
        port powerOut : PowerPort;
    }

    // --- the machine -------------------------------------------------------
    part def CoffeeMachine {
        attribute width_mm : Real = 280;

        part tank : WaterTank;
        part pump : WaterPump;
        part heater : Thermoblock;
        part hopper : BeanHopper;
        part grinder : Grinder;
        part brewUnit : BrewUnit;
        part wand : SteamWand;
        part controller : ControlBoard;
        part display : TouchDisplay;
        part psu : MainsSupply;

        // Water and steam
        connect tank.waterOut to pump.waterIn;
        connect pump.waterOut to heater.waterIn;
        connect heater.hotWaterOut to brewUnit.waterIn;
        connect heater.steamOut to wand.steamIn;

        // Beans and grounds
        connect hopper.beansOut to grinder.beansIn;
        connect grinder.groundsOut to brewUnit.groundsIn;

        // Mains
        connect psu.powerOut to controller.powerIn;

        // Control looms, each carrying the same contract
        connection : ControlLink connect controller.waterCtrl to pump.ctrlIn;
        connection : ControlLink connect controller.waterCtrl to heater.ctrlIn;
        connection : ControlLink connect controller.coffeeCtrl to grinder.ctrlIn;
        connection : ControlLink connect controller.coffeeCtrl to brewUnit.ctrlIn;
        connection : ControlLink connect controller.milkCtrl to wand.ctrlIn;
        connection : ControlLink connect controller.uiLink to display.ctrlOut;
    }

    // The machine itself, named once so deployment.sysml can allocate to the
    // boards inside it: a reference into a definition has to start at a usage.
    part machine : CoffeeMachine;
}
