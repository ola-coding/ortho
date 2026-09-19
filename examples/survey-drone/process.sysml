// Process view: one scenario — a whole survey mission.
//
// The operator marks the area in the app, the route planner onboard turns it
// into survey legs, and once approved the flight firmware flies them in Auto
// mode while video and progress stream back, then returns and lands. One
// scenario per file, because the view renders every message it can see — the
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

    // What the messages carry. Values, not things, so attribute defs; the
    // MAVLink ones are named after the protocol's own message types.
    attribute def AreaPolygon;
    attribute def Waypoints;
    attribute def MissionItems;
    attribute def MissionCount;
    attribute def MissionRequestInt;
    attribute def MissionItemInt;
    attribute def MissionAck;
    attribute def CommandLong;
    attribute def SetMode;
    attribute def H265Stream;
    attribute def MissionItemReached;
    attribute def MissionCurrent;
    attribute def SurveyProgress;

    action def SurveyMission {
        part operator;
        part app : Software::Ground::SurveyPlanner;
        part planner : Software::Onboard::Autonomy::RoutePlanner;
        part bridge : Software::Middleware::MavlinkBridge;
        part mavlink : Software::Flight::MavlinkInterface;
        part mission : Software::Flight::MissionExecutor;
        part video : Software::Onboard::Perception::VideoStreamer;

        // Planning, on the ground and in the air.
        message markArea from operator to app;
        then message surveyRequest of AreaPolygon from app to planner;
        then message computeSurveyLegs from planner to planner;
        then message routePreview of Waypoints from planner to app;
        then message approve from operator to app;
        then message startSurvey from app to planner;

        // Upload, in MAVLink's mission protocol: the vehicle asks for each
        // item in turn, and acknowledges the set once it has them all.
        then message uploadMission of MissionItems from planner to bridge;
        then message missionCount of MissionCount from bridge to mavlink;
        then message requestItem of MissionRequestInt from mavlink to bridge;
        then message missionItem of MissionItemInt from bridge to mavlink;
        then message storeItems from mavlink to mission;
        then message missionAck of MissionAck from mavlink to bridge;

        // Arm, then Auto: two commands, as on any ArduPilot vehicle.
        then message armThrottle of CommandLong from bridge to mavlink;
        then message setAutoMode of SetMode from bridge to mavlink;
        then message startAuto from mavlink to mission;

        // Flying it, and saying so.
        then message flySurveyLegs from mission to mission;
        then message liveVideo of H265Stream from video to app;
        then message waypointReached of MissionItemReached from mission to mavlink;
        then message missionCurrent of MissionCurrent from mavlink to bridge;
        then message progress of SurveyProgress from bridge to app;
        then message returnToLaunch from mission to mission;
        then message missionFinished from mission to mavlink;
        then message missionComplete from mavlink to bridge;
        then message surveyComplete from bridge to app;
    }
}
