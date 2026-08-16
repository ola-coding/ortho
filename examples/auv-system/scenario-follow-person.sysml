// Process view scenario: follow a person with obstacle avoidance.
// Launch, get live video on the radio controller, tap the person to follow;
// the aircraft then tracks autonomously while avoiding obstacles.
package FollowPersonScenario {

    import Software::*;

    action def FollowPerson {
        part pilot;
        part rcApp : Software::PilotApp;
        part flightControl : Software::FlightControlStack;
        part cameraCtrl : Software::CameraController;
        part targetTracking : Software::TargetTrackingEngine;
        part perception : Software::PerceptionPipeline;

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
