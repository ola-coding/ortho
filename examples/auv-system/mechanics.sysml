// Mechanical domain: airframe motion, the camera mount and the pilot's sticks.
//
// Owns the physical budget — mass, geometry, mounting and the MountPort
// interface at system level. Thin by comparison with the electronic domain,
// which is honest for an aircraft of this class: most of the design content is
// electronic, and mechanics contributes motion and structure.
package Mechanics {

    import System::*;

    // --- payload stabilisation --------------------------------------------
    part def ThreeAxisGimbal :> System::Gimbal {
        attribute axes : Real = 3;
    }

    // --- propulsion -------------------------------------------------------
    part def BrushlessMotor :> System::Motor;

    // Folding blades: the design choice that lets the aircraft stow flat.
    part def FoldingPropeller :> System::Propeller;

    // --- pilot input ------------------------------------------------------
    // Gimbaled sticks: a mechanical input device with an electrical output.
    part def GimbalStick :> System::ControlSticks;
}
