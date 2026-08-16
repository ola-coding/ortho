// System requirements for the bean-to-cup coffee machine.
// Self-contained: subjects are untyped so the requirements package has no
// dependency on the design packages (the design satisfies these, not vice versa).
package Requirements {

    requirement def BrewPressure {
        subject pump;
        attribute minPressure_bar : Real = 15;
    }

    requirement def BrewTemperature {
        subject heater;
        attribute targetTemp_C : Real = 92;
    }

    requirement def WaterCapacity {
        subject tank;
        attribute minCapacity_l : Real = 1.9;
    }

    requirement def BeanCapacity {
        subject hopper;
        attribute minCapacity_g : Real = 280;
    }

    requirement def OneTouchMilkBeverages {
        subject machine;
        attribute mode : String = "single_press_milk_beverages";
    }

    requirement pressureReq : BrewPressure;
    requirement temperatureReq : BrewTemperature;
    requirement waterReq : WaterCapacity;
    requirement beanReq : BeanCapacity;
    requirement oneTouchReq : OneTouchMilkBeverages;
}
