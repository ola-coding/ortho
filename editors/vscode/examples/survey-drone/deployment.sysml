// Deployment view: which computer runs which software.
package Deployment {
    doc /* Declares nothing of its own: its whole content is the mapping between
         * the parts in implementation.sysml and the parts in physical.sysml, so
         * it is rendered together with both. */

    private import Software::*;
    private import Hardware::*;
    private import StandardViewDefinitions::*;

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
