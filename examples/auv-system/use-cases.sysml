// Scenarios: operating the AUV system, focused on following a person.
// The subject is the system-level assembly, not a domain component.
package UseCases {

    import System::*;

    part def Pilot;
    part def TrackedPerson;

    use case def OperateDrone {
        subject system : System::AuvSystem;
        actor pilot : Pilot;
    }

    use case launchAircraft {
        subject system : System::AuvSystem;
        actor pilot : Pilot;
    }

    use case viewLiveVideo {
        subject system : System::AuvSystem;
        actor pilot : Pilot;
    }

    use case selectTarget {
        subject system : System::AuvSystem;
        actor pilot : Pilot;
    }

    use case followPerson : OperateDrone {
        actor person : TrackedPerson;
        include use case selectTarget;
        include use case avoidObstacles;
    }

    use case avoidObstacles {
        subject system : System::AuvSystem;
    }
}
