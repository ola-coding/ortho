// Physical view: the product itself, mechanics, electronics and cabling.
package Hardware {
    doc /* Topology only: which part connects, feeds or wires to which; never
         * where a part sits or how big it is. */

    // A part performs the functions it does itself. The control board performs
    // its own through the software deployed on it.
    private import Functions::*;
    private import ScalarValues::*;
    private import ISQ::*;
    private import SI::*;
    private import StandardViewDefinitions::*;

    // What flows through the ports. A port def says which way its item goes;
    // the receiving end is the same port conjugated, `~WaterPort`.
    item def Water;
    item def Steam;
    item def Coffee;
    item def Milk;
    item def Beverage;
    item def ElectricPower;
    item def Command;
    item def Status;

    port def WaterPort { out item water : Water; }
    port def SteamPort { out item steam : Steam; }
    port def CoffeePort { out item coffee : Coffee; }
    port def MilkPort { out item milk : Milk; }
    port def BeveragePort { out item beverage : Beverage; }
    port def PowerPort { out item power : ElectricPower; }
    port def ControlPort {
        out item command : Command;
        in item status : Status;
    }

    // A cable is a connection, so every control loom is an `interface` usage
    // typed by the contract it carries.
    interface def ControlLink {
        end controller : ControlPort;
        end device : ~ControlPort;
    }

    enum def BurrKind {
        conicalCeramic;
        flatSteel;
    }

    // --- water path --------------------------------------------------------
    part def WaterTank {
        perform beverage.prepareWater.storeWater;

        attribute capacity : VolumeValue = 1.9 [L];
        port waterOut : WaterPort;
    }

    part def WaterPump {
        perform beverage.prepareWater.pumpWater;

        attribute maxPressure : PressureValue = 1.5E6 [Pa];
        port waterIn : ~WaterPort;
        port waterOut : WaterPort;
        port ctrlIn : ~ControlPort;
    }

    part def Thermoblock {
        perform beverage.prepareWater.heatWater;

        attribute targetTemperature : CelsiusTemperatureValue = 92 ['°C'];
        port waterIn : ~WaterPort;
        port hotWaterOut : WaterPort;
        port steamOut : SteamPort;
        port ctrlIn : ~ControlPort;
    }

    // --- coffee path -------------------------------------------------------
    part def BeanHopper {
        perform beverage.prepareCoffee.storeBeans;

        attribute capacity : MassValue = 280 [g];
        port beansOut : CoffeePort;
    }

    part def Grinder {
        perform beverage.prepareCoffee.grindBeans;

        attribute burrs : BurrKind = BurrKind::conicalCeramic;
        port beansIn : ~CoffeePort;
        port groundsOut : CoffeePort;
        port ctrlIn : ~ControlPort;
    }

    part def BrewUnit {
        doc /* The coffee leaves the machine here. */
        perform beverage.prepareCoffee.extractShot;

        attribute maxDose : MassValue = 16 [g];
        port waterIn : ~WaterPort;
        port groundsIn : ~CoffeePort;
        port coffeeOut : BeveragePort;
        port ctrlIn : ~ControlPort;
    }

    // --- milk --------------------------------------------------------------
    part def MilkTube {
        doc /* A tube the user drops into a milk carton. The carton is not part of
             * the machine, so the pickup end connects to nothing here: it is where
             * the product meets whatever the kitchen provides. */
        perform beverage.prepareMilk.drawMilk;

        port pickup : ~MilkPort;
        port milkOut : MilkPort;
    }

    part def SteamWand {
        doc /* Steam through the wand pulls milk up the tube; nothing pumps it.
             * The milk leaves the machine here. */
        perform beverage.prepareMilk.frothMilk;

        port steamIn : ~SteamPort;
        port milkIn : ~MilkPort;
        port milkOut : BeveragePort;
        port ctrlIn : ~ControlPort;
    }

    // --- electronics -------------------------------------------------------
    part def ControlBoard {
        attribute mcuCores : Integer = 2;
        port powerIn : ~PowerPort;
        port uiLink : ControlPort;
        port waterCtrl : ControlPort;
        port coffeeCtrl : ControlPort;
        port milkCtrl : ControlPort;
    }

    part def TouchDisplay {
        perform beverage.controlMachine.selectBeverage;

        attribute diagonal : LengthValue = 109 [mm];
        port ctrlIn : ~ControlPort;
    }

    part def MainsSupply {
        perform beverage.supplyPower;

        port powerOut : PowerPort;
    }

    // --- the machine -------------------------------------------------------
    // The machine as built: the subject of every use case, opened up.
    part def CoffeeMachine :> SystemUseCases::CoffeeMachine {
        attribute width : LengthValue = 280 [mm];

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
        interface : ControlLink connect controller.uiLink to display.ctrlIn;
    }

    // The machine itself, named once so deployment.sysml can allocate to the
    // boards inside it: a reference into a definition has to start at a usage.
    part machine : CoffeeMachine;

    view 'Physical view — Coffee Machine' : InterconnectionView {
        expose machine;
    }
}
