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

    // What the user asks for, in more detail.
    #refinement dependency MakeBeverage to SystemUseCases::makeCappuccino;
    #refinement dependency MakeBeverage to SystemUseCases::makeEspresso;

    view 'Logical view — Coffee Machine' : GeneralView {
        expose MakeBeverage;
        render asTreeDiagram;
    }
}
