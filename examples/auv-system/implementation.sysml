// Implementation view: the software that realizes the control functions.
//
// Airborne software is split by concern; the ground application depends on
// both flight and perception for what it displays. Nothing here says which
// compute node runs what — that is deployment.
package Software {

    package Flight {
        part def FlightControlStack;
        part def NavigationEngine;
    }

    package Perception {
        part def PerceptionPipeline;
        part def TargetTrackingEngine;
    }

    package Payload {
        part def CameraController;
    }

    package Link {
        part def TelemetryService;
    }

    package Ground {
        import Software::Flight;
        import Software::Perception;
        import Software::Link;

        part def PilotApp;
    }

    // The deployable parts, named once so deployment.sysml can allocate them.
    part sw {
        part flightControl : Flight::FlightControlStack;
        part navigation : Flight::NavigationEngine;
        part perception : Perception::PerceptionPipeline;
        part targetTracking : Perception::TargetTrackingEngine;
        part cameraCtrl : Payload::CameraController;
        part telemetry : Link::TelemetryService;
        part pilotApp : Ground::PilotApp;
    }
}
