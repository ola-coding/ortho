// Implementation view: the software, as modules and their dependencies.
//
// Two stacks and an app. The flight firmware is ArduPilot-style: control
// libraries on a hardware abstraction layer over a real-time kernel, with no
// Linux anywhere beneath it. The onboard applications run one Docker container
// per module and talk only through Zenoh, over a minimal Linux image that
// starts from a secure boot and updates over the air. The operator's app
// reaches them through the same middleware, across the encrypted radio link.
// Which computer runs what is deployment.sysml's business, not this file's.
package Software {

    // The functions this software realizes, said on the module that realizes
    // each one. Nothing here knows which computer runs it: that is deployment.
    private import Functions::*;
    private import ScalarValues::*;
    private import StandardViewDefinitions::*;

    // What the modules offer each other. A module that offers an API declares
    // a port of that type; one that needs it declares the same type
    // conjugated, `~MissionApi`, so the two ends mirror each other.
    //
    // The firmware's modules call each other, as libraries do. The onboard
    // containers never call: they publish and subscribe through Zenoh, so
    // their APIs are topics, and a topic's two ends are a publisher and a
    // subscriber of the same type.
    package Apis {

        // --- firmware, called directly ---------------------------------
        port def SensorApi {
            out attribute imu_g : Real;
            out attribute fix : Boolean;
        }

        port def StateApi {
            out attribute attitude_deg : Real;
            out attribute altitude_m : Real;
        }

        port def TargetApi {
            out attribute waypoint : String;
            in attribute reached : Boolean;
        }

        port def MavlinkApi {
            out attribute telemetry : String;
            in attribute command : String;
            // MAVLink's parameter protocol reads and writes the same values.
            inout attribute parameter : Real;
        }

        port def HalApi {
            inout attribute register : Real;
        }

        port def SchedulerApi {
            in attribute period_ms : Real;
        }

        // --- Zenoh topics, published and subscribed --------------------
        port def VideoTopic {
            out attribute stream : String;
        }

        port def WorldTopic {
            out attribute obstacles : String;
        }

        port def RouteTopic {
            out attribute legs : String;
        }

        port def MissionTopic {
            out attribute items : String;
            in attribute progress : String;
        }

        port def HealthTopic {
            out attribute health : String;
        }

        port def TransportApi {
            inout attribute topic : String;
        }

        port def ContainerApi {
            in attribute image : String;
        }
    }

    // Plans the survey and shows its progress.
    package Ground {
        private import Software::Middleware;

        part def SurveyPlanner {
            perform survey.plan.planMission;
            perform survey.interactWithOperator.presentInformation;
            perform survey.interactWithOperator.acceptOperatorInput;

            port route : ~Apis::RouteTopic;
            port mission : Apis::MissionTopic;
            port video : ~Apis::VideoTopic;
        }
    }

    // One Docker container per module. Modules never call each other: they
    // publish and subscribe through Zenoh.
    package Onboard {
        private import Software::Middleware;

        // Plans against what perception knows of the surroundings.
        package Autonomy {
            private import Software::Onboard::Perception;

            part def RoutePlanner {
                perform survey.plan.planRoute;

                port route : Apis::RouteTopic;
                port world : ~Apis::WorldTopic;
                port mission : ~Apis::MissionTopic;
            }

            part def ObstacleAvoidance {
                perform survey.guideNavigateAndControl.avoidObstacles;

                port world : ~Apis::WorldTopic;
            }
        }

        package Perception {
            part def VideoStreamer {
                perform survey.communicate.streamVideo;

                port video : Apis::VideoTopic;
            }

            part def SensorFusion {
                perform survey.perceive.fuseSensorData;

                port world : Apis::WorldTopic;
            }

            part def WorldModel {
                perform survey.perceive.maintainWorldModel;

                port world : Apis::WorldTopic;
            }
        }

        package Services {
            part def Diagnostics {
                perform survey.superviseSystem.monitorHealth;

                port health : Apis::HealthTopic;
            }

            part def DataLogger {
                perform survey.superviseSystem.recordFlightData;

                port health : ~Apis::HealthTopic;
                port video : ~Apis::VideoTopic;
            }
        }
    }

    package Middleware {
        private import Software::LinuxPlatform;

        // Carries every message between the containers, and to the tablet.
        part def ZenohRouter {
            port transport : Apis::TransportApi;
        }

        // Turns the flight firmware's MAVLink into Zenoh topics, and back.
        part def MavlinkBridge {
            perform survey.communicate.exchangeTelemetry;

            port mavlink : ~Apis::MavlinkApi;
            port mission : ~Apis::MissionTopic;
        }

        // Video capture, encoding and streaming.
        part def GStreamer {
            port transport : ~Apis::TransportApi;
        }

        // Encrypts everything that crosses the radio link.
        part def WireGuardTunnel {
            port transport : ~Apis::TransportApi;
        }
    }

    // The bottom of the onboard Linux stack.
    package LinuxPlatform {
        part def ContainerRuntime {
            port containers : Apis::ContainerApi;
        }

        // A/B system images, so a failed update rolls itself back.
        part def OtaUpdateAgent {
            port containers : ~Apis::ContainerApi;
        }

        part def MinimalLinux {
            port containers : ~Apis::ContainerApi;
        }

        part def SecureBoot;
    }

    // ArduPilot-style firmware for the control computer. Each comment names
    // the ArduPilot libraries the module stands for.
    package Flight {
        private import Software::FlightPlatform;

        // AP_Mission, AC_WPNav
        part def MissionExecutor {
            perform survey.guideNavigateAndControl.guide;

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

            port vehicleState : ~Apis::StateApi;
            port target : ~Apis::TargetApi;
            port hal : ~Apis::HalApi;
        }

        // AP_InertialSensor, AP_GPS, AP_Compass, AP_Baro. Drivers feed the
        // functions above; they realize none of their own.
        part def SensorDrivers {
            port sensors : Apis::SensorApi;
            port hal : ~Apis::HalApi;
        }

        // GCS_MAVLink
        part def MavlinkInterface {
            perform survey.communicate.exchangeTelemetry;

            port mavlink : Apis::MavlinkApi;
            port vehicleState : ~Apis::StateApi;
        }
    }

    // The bottom of the firmware stack — the counterpart of LinuxPlatform, and
    // the reason the flight code is portable across controller boards.
    package FlightPlatform {
        // AP_HAL, AP_HAL_ChibiOS
        part def HardwareAbstraction {
            port hal : Apis::HalApi;
            port scheduler : ~Apis::SchedulerApi;
        }

        // ChibiOS
        part def RealTimeKernel {
            port scheduler : Apis::SchedulerApi;
        }
    }

    // The deployable parts, named once so deployment.sysml can allocate them,
    // and after their modules so the two views read alike. Every module
    // declared above appears here, because a module missing from this list
    // would be a program with nowhere to run. Only the tunnel appears twice: it
    // is fitted at both ends of the link, and each end is deployed on its own
    // computer.
    part sw {
        part missionExecutor : Flight::MissionExecutor;
        part stateEstimator : Flight::StateEstimator;
        part flightControl : Flight::FlightControl;
        part sensorDrivers : Flight::SensorDrivers;
        part mavlinkInterface : Flight::MavlinkInterface;
        part hardwareAbstraction : FlightPlatform::HardwareAbstraction;
        part realTimeKernel : FlightPlatform::RealTimeKernel;
        part routePlanner : Onboard::Autonomy::RoutePlanner;
        part obstacleAvoidance : Onboard::Autonomy::ObstacleAvoidance;
        part videoStreamer : Onboard::Perception::VideoStreamer;
        part sensorFusion : Onboard::Perception::SensorFusion;
        part worldModel : Onboard::Perception::WorldModel;
        part diagnostics : Onboard::Services::Diagnostics;
        part dataLogger : Onboard::Services::DataLogger;
        part zenohRouter : Middleware::ZenohRouter;
        part mavlinkBridge : Middleware::MavlinkBridge;
        part gStreamer : Middleware::GStreamer;
        part airWireGuardTunnel : Middleware::WireGuardTunnel;
        part containerRuntime : LinuxPlatform::ContainerRuntime;
        part otaUpdateAgent : LinuxPlatform::OtaUpdateAgent;
        part minimalLinux : LinuxPlatform::MinimalLinux;
        part secureBoot : LinuxPlatform::SecureBoot;
        part surveyPlanner : Ground::SurveyPlanner;
        part groundWireGuardTunnel : Middleware::WireGuardTunnel;
    }

    view 'Implementation view — Survey Drone' : GeneralView {
        expose Software;
    }
}
