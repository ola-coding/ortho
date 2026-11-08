// Physical view: the product itself — mechanics, electronics and cabling.
//
// Topology only. Which part connects, feeds or wires to which; never where a
// part sits or how big it is. A cable is a connection, so the control looms
// below are `interface` usages, typed by the contract they carry.
package Hardware {

    // Real, Integer and String are value types from the spec's library.
    private import ScalarValues::*;

    // The functions this hardware realizes, said on the part that realizes
    // each one.
    private import Functions::*;
    private import StandardViewDefinitions::*;

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
        perform beverage.prepareWater.storeWater;

        attribute capacity_l : Real = 1.9;
        port waterOut : WaterPort;
    }

    part def WaterPump {
        perform beverage.prepareWater.pumpWater;

        attribute maxPressure_bar : Real = 15;
        port waterIn : WaterPort;
        port waterOut : WaterPort;
        port ctrlIn : DataPort;
    }

    part def Thermoblock {
        perform beverage.prepareWater.heatWater;

        attribute targetTemp_C : Real = 92;
        port waterIn : WaterPort;
        port hotWaterOut : WaterPort;
        port steamOut : SteamPort;
        port ctrlIn : DataPort;
    }

    // --- coffee path -------------------------------------------------------
    part def BeanHopper {
        perform beverage.prepareCoffee.storeBeans;

        attribute capacity_g : Real = 280;
        port beansOut : CoffeePort;
    }

    part def Grinder {
        perform beverage.prepareCoffee.grindBeans;

        attribute burrs : String = "conical_ceramic";
        port beansIn : CoffeePort;
        port groundsOut : CoffeePort;
        port ctrlIn : DataPort;
    }

    part def BrewUnit {
        perform beverage.prepareCoffee.extractShot;
        // The coffee leaves the machine here.
        perform beverage.serveBeverage;

        attribute chamberVolume_g : Real = 16;
        port waterIn : WaterPort;
        port groundsIn : CoffeePort;
        port coffeeOut : BeveragePort;
        port ctrlIn : DataPort;
    }

    // --- milk --------------------------------------------------------------
    // A tube the user drops into a milk carton. The carton is not part of the
    // machine, so the pickup end connects to nothing here: it is where the
    // product meets whatever the kitchen provides.
    part def MilkTube {
        perform beverage.prepareMilk.drawMilk;

        port pickup : BeveragePort;
        port milkOut : BeveragePort;
    }

    part def SteamWand {
        perform beverage.prepareMilk.frothMilk;
        // And the milk, here.
        perform beverage.serveBeverage;

        port steamIn : SteamPort;
        // Steam through the wand pulls milk up the tube; nothing pumps it.
        port milkIn : BeveragePort;
        port milkOut : BeveragePort;
        port ctrlIn : DataPort;
    }

    // --- electronics -------------------------------------------------------
    part def ControlBoard {
        attribute mcuCores : Integer = 2;
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
        part milkTube : MilkTube;
        part wand : SteamWand;
        part controller : ControlBoard;
        part display : TouchDisplay;
        part psu : MainsSupply;

        // Water and steam
        connect tank.waterOut to pump.waterIn;
        connect pump.waterOut to heater.waterIn;
        connect heater.hotWaterOut to brewUnit.waterIn;
        connect heater.steamOut to wand.steamIn;
        connect milkTube.milkOut to wand.milkIn;

        // Beans and grounds
        connect hopper.beansOut to grinder.beansIn;
        connect grinder.groundsOut to brewUnit.groundsIn;

        // Mains
        connect psu.powerOut to controller.powerIn;

        // Control looms, each carrying the same contract
        interface : ControlLink connect controller.waterCtrl to pump.ctrlIn;
        interface : ControlLink connect controller.waterCtrl to heater.ctrlIn;
        interface : ControlLink connect controller.coffeeCtrl to grinder.ctrlIn;
        interface : ControlLink connect controller.coffeeCtrl to brewUnit.ctrlIn;
        interface : ControlLink connect controller.milkCtrl to wand.ctrlIn;
        interface : ControlLink connect controller.uiLink to display.ctrlOut;
    }

    // The machine itself, named once so deployment.sysml can allocate to the
    // boards inside it: a reference into a definition has to start at a usage.
    part machine : CoffeeMachine;

    view 'Physical view — Coffee Machine' : InterconnectionView {
        expose machine;
    }
}
