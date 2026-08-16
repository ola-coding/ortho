// Software domain: the component architecture and its deployment onto the
// airborne and ground compute nodes.
//
// Owns the compute budget — CPU load, memory and latency on each node it is
// allocated to. The allocations below are the deployment: every software part
// lands on a part of the system architecture, never on an electronics
// realization, which is what keeps the domains independent of each other.
package Software {

    import System::*;
    import Requirements::*;

    // --- architecture -----------------------------------------------------
    part def FlightControlStack {
        attribute controlLoop_Hz : Real = 1000;
    }
    part def NavigationEngine {
        attribute modes : String = "GNSS_Vision_Attitude";
    }
    part def PerceptionPipeline {
        attribute feature : String = "omnidirectional_obstacle_avoidance";
    }
    part def TargetTrackingEngine {
        attribute feature : String = "target_tracking_360";
    }
    part def CameraController {
        attribute pipeline : String = "HDR_video";
    }
    part def TelemetryService;
    part def PilotApp {
        attribute ui : String = "liveview_touch_target_selection";
    }

    part sw {
        part flightControl : FlightControlStack;
        part navigation : NavigationEngine;
        part perception : PerceptionPipeline;
        part targetTracking : TargetTrackingEngine;
        part cameraCtrl : CameraController;
        part telemetry : TelemetryService;
        part pilotApp : PilotApp;
    }

    // --- deployment onto the compute nodes --------------------------------
    allocate sw.flightControl to System::Aircraft::fc;
    allocate sw.navigation to System::Aircraft::fc;
    allocate sw.perception to System::Aircraft::fc;
    allocate sw.targetTracking to System::Aircraft::fc;
    allocate sw.cameraCtrl to System::Aircraft::camera;
    allocate sw.telemetry to System::Aircraft::radio;
    allocate sw.pilotApp to System::AuvSystem::controller;

    // --- requirement traces owned by this domain --------------------------
    satisfy Requirements::trackingReq by TargetTrackingEngine;
}
