// Smoke model for the implementation view (packages and imports) and the
// deployment view (software allocated to hardware).
package DroneSystem {

    package Requirements {
        requirement def MassLimit;
    }

    package Structure {
        private import Requirements::*;
        part def Airframe;
    }

    package Physical {
        private import Structure::*;

        part def FlightController;
        part def CompanionComputer;

        part fc : FlightController;
        part cc : CompanionComputer;
    }

    package Software {
        private import Structure::*;
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
