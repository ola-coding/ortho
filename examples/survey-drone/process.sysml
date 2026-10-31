// Process view: one scenario — a whole survey mission.
//
// The operator marks the area in the app, the route planner onboard turns it
// into survey legs, and once approved the flight firmware flies them in Auto
// mode while video and progress stream back, then returns and lands. The
// obstacle-avoidance deviation is a scenario of its own, not a branch of this
// one.
//
// The middle of the diagram is the boundary between the two stacks, and it is
// drawn at MAVLink's own granularity: the bridge uploads a mission the way a
// ground station does — a count, an item request from the vehicle, the item,
// an ack — because that handshake is the interface between them, and hiding it
// behind one arrow would hide the only place the stacks meet.
package SurveyMissionScenario {

    private import Software::*;
    private import StandardViewDefinitions::*;

    // What the messages carry; the MAVLink ones are named after the
    // protocol's own message types.
    item def AreaPolygon;
    item def Waypoints;
    item def MissionItems;
    item def MissionCount;
    item def MissionRequestInt;
    item def MissionItemInt;
    item def MissionAck;
    item def CommandLong;
    item def SetMode;
    item def H265Stream;
    item def MissionItemReached;
    item def MissionCurrent;
    item def SurveyProgress;

    occurrence def SurveyMission {
        part operator;
        part surveyPlanner : Ground::SurveyPlanner;
        part routePlanner : Onboard::Autonomy::RoutePlanner;
        part mavlinkBridge : Middleware::MavlinkBridge;
        part mavlinkInterface : Flight::MavlinkInterface;
        part missionExecutor : Flight::MissionExecutor;
        part videoStreamer : Onboard::Perception::VideoStreamer;

        // Planning, on the ground and in the air.
        message markArea from operator to surveyPlanner;
        then message surveyRequest of AreaPolygon from surveyPlanner to routePlanner;
        then message computeSurveyLegs from routePlanner to routePlanner;
        then message routePreview of Waypoints from routePlanner to surveyPlanner;
        then message approve from operator to surveyPlanner;
        then message startSurvey from surveyPlanner to routePlanner;

        // Upload, in MAVLink's mission protocol: the vehicle asks for each
        // item in turn, and acknowledges the set once it has them all.
        then message uploadMission of MissionItems from routePlanner to mavlinkBridge;
        then message missionCount of MissionCount from mavlinkBridge to mavlinkInterface;
        then message requestItem of MissionRequestInt from mavlinkInterface to mavlinkBridge;
        then message missionItem of MissionItemInt from mavlinkBridge to mavlinkInterface;
        then message storeItems from mavlinkInterface to missionExecutor;
        then message missionAck of MissionAck from mavlinkInterface to mavlinkBridge;

        // Arm, then Auto: two commands, as on any ArduPilot vehicle.
        then message armThrottle of CommandLong from mavlinkBridge to mavlinkInterface;
        then message setAutoMode of SetMode from mavlinkBridge to mavlinkInterface;
        then message startAuto from mavlinkInterface to missionExecutor;

        // Flying it, and saying so.
        then message flySurveyLegs from missionExecutor to missionExecutor;
        then message liveVideo of H265Stream from videoStreamer to surveyPlanner;
        then message waypointReached of MissionItemReached from missionExecutor to mavlinkInterface;
        then message missionCurrent of MissionCurrent from mavlinkInterface to mavlinkBridge;
        then message progress of SurveyProgress from mavlinkBridge to surveyPlanner;
        then message returnToLaunch from missionExecutor to missionExecutor;
        then message missionFinished from missionExecutor to mavlinkInterface;
        then message missionComplete from mavlinkInterface to mavlinkBridge;
        then message surveyComplete from mavlinkBridge to surveyPlanner;
    }

    view 'Process view — Survey Drone' : SequenceView {
        expose SurveyMission;
    }
}
