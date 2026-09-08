// Smoke model for the Logical view: a capability tree.
//
// Nesting is the hierarchy. `holdAltitude` is shared by both branches below it
// and so sits at their lowest common ancestor rather than being duplicated.
package DroneFunctions {

    action def OperateDrone {

        action fly {
            action generateThrust;
            action stabiliseAttitude;
        }

        action navigate {
            action fixPosition;
            action followRoute;
        }

        action holdAltitude;

        action commandDrone {
            action acceptPilotInput;
            action reportTelemetry;
        }
    }
}
