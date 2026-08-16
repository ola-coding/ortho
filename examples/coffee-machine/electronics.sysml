// Electronic domain: the design that realizes the machine's power and control
// blocks.
//
// Owns the electrical budget — voltage, current and power draw, and the
// PowerPort/DataPort parameters at system level. Like the mechanical domain,
// each definition here specializes an architecture block from System, so every
// port is inherited and only the design choices are declared.
package Electronics {

    import System::*;

    // Realization: mains supply, no battery backup.
    part def MainsPowerSupply :> System::PowerSupply;

    // Realization: capacitive touch panel.
    part def TouchDisplay :> System::DisplayModule {
        attribute diagonal_in : Real = 4.3;
    }

    // Realization: one dual-core MCU driving all three mechanical subsystems.
    // This is also the compute node the software domain deploys onto.
    part def MainControlBoard :> System::ControlBoard {
        attribute mcuCores : Real = 2;
    }
}
