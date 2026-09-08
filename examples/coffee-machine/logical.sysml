// Logical view: what the machine does, independent of what does it.
//
// A capability tree. Every function appears exactly once and has exactly one
// parent; nesting in this file is the hierarchy the diagram draws. Nothing
// here names a thermoblock, a pump or a controller — those are realizations,
// and they live in physical.sysml and implementation.sysml.
package Functions {

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
            action doseGrounds;
            action extractShot;
        }

        action prepareMilk {
            action storeMilk;
            action frothMilk;
        }

        action serveBeverage;

        // Control functions. These are the migratory ones — every one of them
        // was a cam, a timer or a bimetallic strip within living memory, and
        // each could move medium again without this tree changing.
        action controlMachine {
            action selectBeverage;
            action sequenceRecipe;
            action regulateTemperature;
            action interlockBrewGroup;
        }
    }
}
