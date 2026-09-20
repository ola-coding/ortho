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

        // The machine holds no milk: it draws what it needs from whatever
        // carton the user stands beside it.
        action prepareMilk {
            action drawMilk;
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

    // One beverage being made. The tree above says what is done; the parts
    // that realize a function point into this usage, because a `perform`
    // chain has to start at one.
    action beverage : MakeBeverage;

    // Looking after the machine is the second thing its owner asks of it, and
    // descaling is the use case that asks. Not a step in making a beverage:
    // the machine runs this cycle instead of brewing, so it is a capability of
    // its own rather than a branch of the tree above.
    action def MaintainMachine {

        // The machine knows when it is furred up; the owner has to be told.
        action signalDescaleDue;

        // Pumping and heating are the beverage functions over again; what is
        // new is sequencing them with descaler instead of coffee.
        action runDescaleCycle;
    }

    action maintenance : MaintainMachine;
}
