// Implementation view: the software, as modules and their dependencies.
//
// Two stacks and an app. The flight firmware is ArduPilot-style: control
// libraries on a hardware abstraction layer over a real-time kernel, with no
// Linux anywhere beneath it. The onboard applications run one Docker container
// per module and talk only through Zenoh, on a stock minimal Linux image, which
// is bought rather than built and so is not a module here. The operator's app
// reaches them through the same middleware, across the encrypted radio link.
// Which computer runs what is deployment.sysml's business, not this file's.

package Apis {
    doc /* What the modules offer each other, and what crosses. A module that
         * offers an API declares a port of that type; one that needs it
         * declares the same type conjugated, `~MavlinkApi`. The onboard
         * containers never call each other: they publish and subscribe
         * through Zenoh, so their APIs are topics, with a publisher at one
         * end and subscribers at the other. */
    private import ScalarValues::*;
    private import ISQ::*;

    // What crosses: the MAVLink ones are named after the protocol's own
    // message types. process.sysml sends the same items.
    item def AreaPolygon;
    item def Waypoint;
    item def Waypoints {
        item waypoints : Waypoint [1..*];
    }
    item def MissionItems;
    item def MissionAck;
    item def CommandLong;
    item def SetMode;
    item def MissionItemReached;
    item def MissionCurrent;
    item def SurveyProgress;
    item def H265Stream;
    item def ObstacleMap;
    item def HealthReport;

    // --- Zenoh topics, published and subscribed ---------------------
    port def SurveyTopic {
        out item area : AreaPolygon;
        out attribute approved : Boolean;
    }

    port def RouteTopic {
        out item route : Waypoints;
    }

    port def MissionTopic {
        out item mission : MissionItems;
    }

    port def ProgressTopic {
        out item progress : SurveyProgress;
    }

    port def VideoTopic {
        out item video : H265Stream;
    }

    port def WorldTopic {
        out item obstacles : ObstacleMap;
    }

    port def HealthTopic {
        out item health : HealthReport;
    }

    port def TransportApi {
        inout attribute topic : String;
    }

    // --- firmware, called directly ----------------------------------
    port def MavlinkApi {
        in item mission : MissionItems;
        in item command : CommandLong;
        in item mode : SetMode;
        out item ack : MissionAck;
        out item current : MissionCurrent;
        // MAVLink's parameter protocol reads and writes the same values.
        inout attribute parameter : Real;
    }

    port def MissionApi {
        in item mission : MissionItems;
        in attribute auto : Boolean;
        out item reached : MissionItemReached;
    }

    port def TargetApi {
        out item target : Waypoint;
        in attribute reached : Boolean;
    }

    port def StateApi {
        out attribute attitude : AngularMeasureValue;
        out attribute altitude : LengthValue;
    }

    port def SensorApi {
        out attribute acceleration : AccelerationValue;
        out attribute fix : Boolean;
    }

    port def HalApi {
        inout attribute register : Integer;
    }

    port def SchedulerApi {
        in attribute period : DurationValue;
    }
}

package Ground {
    doc /* The operator's app, on the tablet. */
    private import Functions::*;
    private import Middleware;

    part def SurveyPlanner {
        doc /* Plans the survey with the operator and shows its progress. */
        perform survey.interactWithOperator.planMission;
        perform survey.interactWithOperator.presentProgress;

        port request : Apis::SurveyTopic;
        port route : ~Apis::RouteTopic;
        port progress : ~Apis::ProgressTopic;
        port video : ~Apis::VideoTopic;
        port health : ~Apis::HealthTopic;
    }
}

package Onboard {
    doc /* One Docker container per module, on the GPU computer. */
    private import Functions::*;
    private import Middleware;

    // Plans against what perception knows of the surroundings.
    part def RoutePlanner {
        perform survey.guideNavigateAndControl.planRoute;

        port request : ~Apis::SurveyTopic;
        port route : Apis::RouteTopic;
        port mission : Apis::MissionTopic;
        port world : ~Apis::WorldTopic;
    }

    part def ObstacleAvoidance {
        perform survey.guideNavigateAndControl.avoidObstacles;

        port world : ~Apis::WorldTopic;
    }

    part def VideoStreamer {
        doc /* Captures, encodes and streams the camera's video, on GStreamer. */
        perform survey.communicate.streamVideo;

        port video : Apis::VideoTopic;
    }

    part def WorldModel {
        doc /* Fuses video and flight state into a map of the obstacles around. */
        perform survey.perceive.maintainWorldModel;

        port video : ~Apis::VideoTopic;
        port world : Apis::WorldTopic;
    }

    part def Diagnostics {
        doc /* Monitors every module's health and records the flight. */
        perform survey.superviseSystem;

        port health : Apis::HealthTopic;
        port video : ~Apis::VideoTopic;
    }
}

package Middleware {
    doc /* What the onboard containers and the operator's app talk through. */
    private import Functions::*;
    part def ZenohRouter {
        doc /* Carries every message between the containers, and to the tablet. */
        port transport : Apis::TransportApi;
    }

    part def MavlinkBridge {
        doc /* Turns the flight firmware's MAVLink into Zenoh topics, and back. */
        perform survey.communicate.exchangeTelemetry;

        port mission : ~Apis::MissionTopic;
        port progress : Apis::ProgressTopic;
        port mavlink : ~Apis::MavlinkApi;
    }

    part def WireGuardTunnel {
        doc /* Encrypts everything that crosses the radio link. */
        port transport : ~Apis::TransportApi;
    }
}

package Flight {
    doc /* ArduPilot-style firmware for the control computer. Each comment
         * names the ArduPilot libraries the module stands for. */
    private import Functions::*;
    private import FlightPlatform;

    // AP_Mission, AC_WPNav
    part def MissionExecutor {
        perform survey.guideNavigateAndControl.guide;

        port mission : Apis::MissionApi;
        port target : Apis::TargetApi;
        port vehicleState : ~Apis::StateApi;
    }

    // AP_AHRS, AP_NavEKF3
    part def StateEstimator {
        perform survey.guideNavigateAndControl.navigate;

        port vehicleState : Apis::StateApi;
        port sensors : ~Apis::SensorApi;
    }

    // AC_AttitudeControl, AC_PosControl, AP_Motors
    part def FlightControl {
        perform survey.guideNavigateAndControl.control;

        port target : ~Apis::TargetApi;
        port vehicleState : ~Apis::StateApi;
        port hal : ~Apis::HalApi;
    }

    // GCS_MAVLink
    part def MavlinkInterface {
        perform survey.communicate.exchangeTelemetry;

        port mavlink : Apis::MavlinkApi;
        port mission : ~Apis::MissionApi;
        port vehicleState : ~Apis::StateApi;
    }
}

package FlightPlatform {
    doc /* The bottom of the firmware stack, and the reason the flight code is
         * portable across controller boards. */

    // AP_HAL, AP_HAL_ChibiOS, and the sensor drivers on it: AP_InertialSensor,
    // AP_GPS, AP_Compass, AP_Baro.
    part def HardwareAbstraction {
        port sensors : Apis::SensorApi;
        port hal : Apis::HalApi;
        port scheduler : ~Apis::SchedulerApi;
    }

    // ChibiOS
    part def RealTimeKernel {
        port scheduler : Apis::SchedulerApi;
    }
}

view 'Implementation view — Survey Drone' : StandardViewDefinitions::GeneralView {
    expose Ground;
    expose Onboard;
    expose Middleware;
    expose Flight;
    expose FlightPlatform;
}
