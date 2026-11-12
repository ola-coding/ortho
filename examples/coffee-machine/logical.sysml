// Logical view: what the machine does, independent of what does it.
package Functions {
    doc /* A capability tree. Every function appears exactly once and has exactly
         * one parent. Nothing here names a thermoblock, a pump or a controller. */
    private import StandardViewDefinitions::*;
    private import Views::*;
    private import ModelingMetadata::*;

    // Branches in the order a beverage is made, then what keeps it going.
    action def MakeBeverage {

        // Energy and material functions. These are medium-bound: no software
        // will ever hold a litre of water or fracture a bean.
        action prepareWater {
            action storeWater;
            action pumpWater;
            action heatWater;
        }

        action prepareCoffee {
            action storeBeans;
            action grindBeans;
            action extractShot;
        }

        // The machine holds no milk: it draws what it needs from whatever carton
        // the user stands beside it.
        action prepareMilk {
            action drawMilk;
            action frothMilk;
        }

        action supplyPower;

        // Control functions. These are the migratory ones: every one of them was
        // a cam, a timer or a bimetallic strip within living memory.
        action controlMachine {
            action selectBeverage;
            action sequenceRecipe;
            action regulateTemperature;
        }
    }

    // One beverage being made. A `perform` chain has to start at a usage, so
    // the parts that realize a function point into this one.
    action beverage : MakeBeverage;

    // Not a step in making a beverage: the machine runs this cycle instead of
    // brewing, so it is a capability of its own.
    action def MaintainMachine {
        // The machine knows when it is furred up; the owner has to be told.
        action signalDescaleDue;
        // Pumping and heating over again, with descaler instead of coffee.
        action runDescaleCycle;
    }

    action maintenance : MaintainMachine;

    // What the user asks for, in more detail.
    #refinement dependency MakeBeverage to SystemUseCases::makeCappuccino;
    #refinement dependency MakeBeverage to SystemUseCases::makeEspresso;
    #refinement dependency MaintainMachine to SystemUseCases::descale;

    view 'Logical view — Coffee Machine' : GeneralView {
        expose MakeBeverage;
        expose MaintainMachine;
        expose SystemUseCases::makeCappuccino;
        expose SystemUseCases::makeEspresso;
        expose SystemUseCases::descale;
        render asTreeDiagram;
    }
}
