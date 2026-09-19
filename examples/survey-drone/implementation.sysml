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

    // Plans the survey and shows its progress.
    package Ground {
        private import Software::Middleware;

        part def SurveyPlanner {
            perform Functions::survey.plan.planMission;
            perform Functions::survey.interactWithOperator.presentInformation;
            perform Functions::survey.interactWithOperator.acceptOperatorInput;
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
            perform Functions::survey.plan.planRoute;
        }
            part def ObstacleAvoidance {
            perform Functions::survey.guideNavigateAndControl.avoidObstacles;
        }
        }

        package Perception {
            part def VideoStreamer {
            perform Functions::survey.communicate.streamVideo;
        }
            part def SensorFusion {
            perform Functions::survey.perceive.fuseSensorData;
        }
            part def WorldModel {
            perform Functions::survey.perceive.maintainWorldModel;
        }
        }

        package Services {
            part def Diagnostics {
            perform Functions::survey.superviseSystem.monitorHealth;
        }
            part def DataLogger {
            perform Functions::survey.superviseSystem.recordFlightData;
        }
        }
    }

    package Middleware {
        private import Software::LinuxPlatform;

        // Carries every message between the containers, and to the tablet.
        part def ZenohRouter;
        // Turns the flight firmware's MAVLink into Zenoh topics, and back.
        part def MavlinkBridge {
            perform Functions::survey.communicate.exchangeTelemetry;
        }
        // Video capture, encoding and streaming.
        part def GStreamer;
        // Encrypts everything that crosses the radio link.
        part def WireGuardTunnel;
    }

    // The bottom of the onboard Linux stack.
    package LinuxPlatform {
        part def ContainerRuntime;
        // A/B system images, so a failed update rolls itself back.
        part def OtaUpdateAgent;
        part def MinimalLinux;
        part def SecureBoot;
    }

    // ArduPilot-style firmware for the control computer. Each comment names
    // the ArduPilot libraries the module stands for.
    package Flight {
        private import Software::FlightPlatform;

        // AP_Mission, AC_WPNav
        part def MissionExecutor {
            perform Functions::survey.guideNavigateAndControl.guide;
        }

        // AP_AHRS, AP_NavEKF3
        part def StateEstimator {
            perform Functions::survey.guideNavigateAndControl.navigate;
        }

        // AC_AttitudeControl, AC_PosControl, AP_Motors
        part def FlightControl {
            perform Functions::survey.guideNavigateAndControl.control;
        }

        // AP_InertialSensor, AP_GPS, AP_Compass, AP_Baro. Drivers feed the
        // functions above; they realize none of their own.
        part def SensorDrivers;

        // GCS_MAVLink
        part def MavlinkInterface {
            perform Functions::survey.communicate.exchangeTelemetry;
        }
    }

    // The bottom of the firmware stack — the counterpart of LinuxPlatform, and
    // the reason the flight code is portable across controller boards.
    package FlightPlatform {
        part def HardwareAbstraction;  // AP_HAL, AP_HAL_ChibiOS
        part def RealTimeKernel;       // ChibiOS
    }

    // The deployable parts, named once so deployment.sysml can allocate them.
    // Every module declared above appears here, because a module missing from
    // this list would be a program with nowhere to run. Only the tunnel appears
    // twice: it is fitted at both ends of the link, and each end is deployed
    // on its own computer.
    part sw {
        part mission : Flight::MissionExecutor;
        part estimator : Flight::StateEstimator;
        part control : Flight::FlightControl;
        part sensors : Flight::SensorDrivers;
        part mavlink : Flight::MavlinkInterface;
        part hal : FlightPlatform::HardwareAbstraction;
        part rtos : FlightPlatform::RealTimeKernel;
        part planner : Onboard::Autonomy::RoutePlanner;
        part avoidance : Onboard::Autonomy::ObstacleAvoidance;
        part video : Onboard::Perception::VideoStreamer;
        part fusion : Onboard::Perception::SensorFusion;
        part world : Onboard::Perception::WorldModel;
        part diagnostics : Onboard::Services::Diagnostics;
        part logger : Onboard::Services::DataLogger;
        part zenoh : Middleware::ZenohRouter;
        part bridge : Middleware::MavlinkBridge;
        part gstreamer : Middleware::GStreamer;
        part airTunnel : Middleware::WireGuardTunnel;
        part runtime : LinuxPlatform::ContainerRuntime;
        part ota : LinuxPlatform::OtaUpdateAgent;
        part linux : LinuxPlatform::MinimalLinux;
        part secureBoot : LinuxPlatform::SecureBoot;
        part app : Ground::SurveyPlanner;
        part groundTunnel : Middleware::WireGuardTunnel;
    }
}
