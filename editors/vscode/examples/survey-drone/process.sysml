// Process view: one scenario, a whole survey mission.
package SurveyMissionScenario {
    doc /* The operator marks the area in the app, the route planner onboard
         * turns it into survey legs, and once approved the flight firmware flies
         * them in Auto mode while progress streams back, then returns and lands.
         * The obstacle-avoidance deviation would be a scenario of its own. */

    private import Software::*;
    private import Software::Apis::*;
    private import StandardViewDefinitions::*;

    // The lifelines are the operator and the modules of implementation.sysml;
    // each message is an item one of their APIs carries.
    occurrence def SurveyMission {
        part operator : SystemUseCases::Operator;
        part planner : Ground::SurveyPlanner;
        part routePlanner : Onboard::RoutePlanner;
        part bridge : Middleware::MavlinkBridge;
        part mavlink : Flight::MavlinkInterface;
        part executor : Flight::MissionExecutor;

        // Planning, on the ground and in the air.
        message markArea from operator to planner;
        then message surveyRequest of AreaPolygon from planner to routePlanner;
        then message computeSurveyLegs from routePlanner to routePlanner;
        then message routePreview of Waypoints from routePlanner to planner;
        then message approve from operator to planner;
        then message startSurvey from planner to routePlanner;

        // Upload. MAVLink's mission protocol hands the items over one at a
        // time, each one asked for by the vehicle; drawn here as one message.
        then message uploadMission of MissionItems from routePlanner to bridge;
        then message missionUpload of MissionItems from bridge to mavlink;
        then message storeMission of MissionItems from mavlink to executor;
        then message missionAck of MissionAck from mavlink to bridge;

        // Arm, then Auto: two commands, as on any ArduPilot vehicle.
        then message arm of CommandLong from bridge to mavlink;
        then message setAutoMode of SetMode from bridge to mavlink;
        then message startAuto from mavlink to executor;

        // Flying it, and saying so.
        then message flySurveyLegs from executor to executor;
        then message waypointReached of MissionItemReached from executor to mavlink;
        then message missionCurrent of MissionCurrent from mavlink to bridge;
        then message progress of SurveyProgress from bridge to planner;
        then message returnToLaunch from executor to executor;
        then message missionFinished from executor to mavlink;
        then message missionComplete from mavlink to bridge;
        then message surveyComplete from bridge to planner;
    }

    view 'Process view — Survey Drone' : SequenceView {
        expose SurveyMission;
    }
}
