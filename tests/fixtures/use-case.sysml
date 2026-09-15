// Smoke model for the use case view of a small quadcopter.
package DroneUseCases {

    part def Pilot;
    part def Drone;

    use case def FlyMission {
        subject drone : Drone;
        actor pilot : Pilot;
    }

    use case flyMission : FlyMission {
        include use case avoidObstacles;
        include use case returnToHome;
    }

    use case avoidObstacles {
        subject drone : Drone;
    }

    use case returnToHome {
        subject drone : Drone;
        actor pilot : Pilot;
    }

    use case capturePhotos {
        subject drone : Drone;
        actor pilot : Pilot;
    }
}
