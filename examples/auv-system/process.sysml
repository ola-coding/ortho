// Process view: one scenario — the software following a person.
//
// Launch, get live video on the handset, tap the person to follow; the aircraft
// then tracks autonomously while staying clear of obstacles. One scenario per
// file, because the view renders every message it can see.
package FollowPersonScenario {

    import Software::*;

    action def FollowPerson {
        part pilot;
        part rcApp : Software::Ground::PilotApp;
        part flightControl : Software::Flight::FlightControlStack;
        part cameraCtrl : Software::Payload::CameraController;
        part targetTracking : Software::Perception::TargetTrackingEngine;
        part perception : Software::Perception::PerceptionPipeline;

        message launch from pilot to rcApp;
        then message takeOff from rcApp to flightControl;
        then message liveVideo of VideoStream from cameraCtrl to rcApp;
        then message tapPerson from pilot to rcApp;
        then message trackTarget of TargetBox from rcApp to targetTracking;
        then message followSetpoints from targetTracking to flightControl;
        then message obstacleMap of DepthGrid from perception to flightControl;
        then message avoidanceManeuver from flightControl to flightControl;
        then message trackingStatus from targetTracking to rcApp;
    }
}
