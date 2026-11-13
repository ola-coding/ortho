// Deployment view: which computer runs which software.
package Deployment {
    doc /* The mapping between the modules of implementation.sysml and the parts
         * of physical.sysml, so it is rendered together with both. */

    private import Hardware::*;
    private import StandardViewDefinitions::*;

    // The deployable software: one usage of each module, named after it, in the
    // order of implementation.sysml's packages, for the allocations below to
    // start from. Every module appears here: one missing would be a program with
    // nowhere to run. The tunnel appears twice, once for each end of the link.
    part sw {
        part surveyPlanner : Ground::SurveyPlanner;
        part routePlanner : Onboard::RoutePlanner;
        part obstacleAvoidance : Onboard::ObstacleAvoidance;
        part videoStreamer : Onboard::VideoStreamer;
        part worldModel : Onboard::WorldModel;
        part diagnostics : Onboard::Diagnostics;
        part zenohRouter : Middleware::ZenohRouter;
        part mavlinkBridge : Middleware::MavlinkBridge;
        part groundWireGuardTunnel : Middleware::WireGuardTunnel;
        part airWireGuardTunnel : Middleware::WireGuardTunnel;
        part missionExecutor : Flight::MissionExecutor;
        part stateEstimator : Flight::StateEstimator;
        part flightControl : Flight::FlightControl;
        part mavlinkInterface : Flight::MavlinkInterface;
        part hardwareAbstraction : FlightPlatform::HardwareAbstraction;
        part realTimeKernel : FlightPlatform::RealTimeKernel;
    }

    // The operator's app runs on the tablet, behind its own end of the tunnel.
    allocate sw.surveyPlanner to system.ground.tablet;
    allocate sw.groundWireGuardTunnel to system.ground.tablet;

    // Every onboard container shares the GPU computer, with the middleware they
    // talk through.
    allocate sw.routePlanner to system.drone.companion;
    allocate sw.obstacleAvoidance to system.drone.companion;
    allocate sw.videoStreamer to system.drone.companion;
    allocate sw.worldModel to system.drone.companion;
    allocate sw.diagnostics to system.drone.companion;
    allocate sw.zenohRouter to system.drone.companion;
    allocate sw.mavlinkBridge to system.drone.companion;
    allocate sw.airWireGuardTunnel to system.drone.companion;

    // The flight firmware has the control computer to itself, down to the kernel.
    allocate sw.missionExecutor to system.drone.fc;
    allocate sw.stateEstimator to system.drone.fc;
    allocate sw.flightControl to system.drone.fc;
    allocate sw.mavlinkInterface to system.drone.fc;
    allocate sw.hardwareAbstraction to system.drone.fc;
    allocate sw.realTimeKernel to system.drone.fc;

    view 'Deployment view — Survey Drone' : GeneralView {
        expose Deployment;
    }
}
