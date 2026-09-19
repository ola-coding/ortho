// Smoke model for the implementation view (packages and imports) and the
// deployment view (software allocated to hardware).
package DroneSystem {

    package Requirements {
        requirement def MassLimit;
    }

    package Logical {
        private import Requirements::*;
        part def Airframe;
    }

    package Physical {
        private import Logical::*;

        part def FlightController;
        part def CompanionComputer;

        part fc : FlightController;
        part cc : CompanionComputer;
    }

    package Software {
        private import Logical::*;
        private import Physical::*;

        part flightStack {
            part attitudeControl;
            part navigation;
            part perception;
            part telemetry;
        }

        allocate flightStack.attitudeControl to Physical::fc;
        allocate flightStack.telemetry to Physical::fc;
        allocate flightStack.navigation to Physical::cc;
        allocate flightStack.perception to Physical::cc;
    }
}
