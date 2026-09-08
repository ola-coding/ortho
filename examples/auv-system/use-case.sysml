// Use case view: what a pilot asks the system for.
//
// The subject is untyped: naming an airframe part would be realization detail,
// and this view carries none.
package UseCases {

    part def Pilot;
    part def TrackedPerson;

    use case def OperateDrone {
        subject AuvSystem;
        actor pilot : Pilot;
    }

    use case launchAircraft : OperateDrone;

    use case viewLiveVideo : OperateDrone;

    use case selectTarget : OperateDrone;

    use case avoidObstacles : OperateDrone;

    // Following a person is something a pilot asks for; picking the target and
    // staying clear of obstacles are part of what they experience doing it.
    use case followPerson : OperateDrone {
        actor person : TrackedPerson;
        include use case selectTarget;
        include use case avoidObstacles;
    }
}
