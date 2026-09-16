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

    // Plans the survey and shows its progress.
    package Ground {
        import Software::Middleware;

        part def SurveyPlanner;
    }

    // One Docker container per module. Modules never call each other: they
    // publish and subscribe through Zenoh.
    package Onboard {
        import Software::Middleware;

        // Plans against what perception knows of the surroundings.
        package Autonomy {
            import Software::Onboard::Perception;

            part def RoutePlanner;
            part def ObstacleAvoidance;
        }

        package Perception {
            part def VideoStreamer;
            part def SensorFusion;
            part def WorldModel;
        }

        package Services {
            part def Diagnostics;
            part def DataLogger;
        }
    }

    package Middleware {
        import Software::LinuxPlatform;

        // Carries every message between the containers, and to the tablet.
        part def ZenohRouter;
        // Turns the flight firmware's MAVLink into Zenoh topics, and back.
        part def MavlinkBridge;
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
        import Software::FlightPlatform;

        part def MissionExecutor;   // AP_Mission, AC_WPNav
        part def StateEstimator;    // AP_AHRS, AP_NavEKF3
        part def FlightControl;     // AC_AttitudeControl, AC_PosControl, AP_Motors
        part def SensorDrivers;     // AP_InertialSensor, AP_GPS, AP_Compass, AP_Baro
        part def MavlinkInterface;  // GCS_MAVLink
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
