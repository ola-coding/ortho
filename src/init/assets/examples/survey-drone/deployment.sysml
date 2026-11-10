// Deployment view: which computer runs which software.
//
// This file declares nothing of its own. Its whole content is the mapping
// between the parts in implementation.sysml and the parts in physical.sysml,
// which is why it must be rendered together with both. Every module named in
// implementation.sysml is allocated here — a computer's own platform included,
// since a kernel runs on a processor like anything else.
package Deployment {

    private import Software::*;
    private import Hardware::*;
    private import StandardViewDefinitions::*;

    // The flight firmware has the control computer to itself: the control
    // libraries, the drivers that feed them, and the kernel underneath.
    allocate sw.missionExecutor to system.drone.fc;
    allocate sw.stateEstimator to system.drone.fc;
    allocate sw.flightControl to system.drone.fc;
    allocate sw.sensorDrivers to system.drone.fc;
    allocate sw.mavlinkInterface to system.drone.fc;
    allocate sw.hardwareAbstraction to system.drone.fc;
    allocate sw.realTimeKernel to system.drone.fc;

    // Every onboard container shares the GPU computer, with the middleware
    // and the Linux platform they stand on.
    allocate sw.routePlanner to system.drone.companion;
    allocate sw.obstacleAvoidance to system.drone.companion;
    allocate sw.videoStreamer to system.drone.companion;
    allocate sw.sensorFusion to system.drone.companion;
    allocate sw.worldModel to system.drone.companion;
    allocate sw.diagnostics to system.drone.companion;
    allocate sw.dataLogger to system.drone.companion;
    allocate sw.zenohRouter to system.drone.companion;
    allocate sw.mavlinkBridge to system.drone.companion;
    allocate sw.gStreamer to system.drone.companion;
    allocate sw.airWireGuardTunnel to system.drone.companion;
    allocate sw.containerRuntime to system.drone.companion;
    allocate sw.otaUpdateAgent to system.drone.companion;
    allocate sw.minimalLinux to system.drone.companion;
    allocate sw.secureBoot to system.drone.companion;

    // The operator's app runs on the tablet, behind its own end of the tunnel.
    allocate sw.surveyPlanner to system.ground.tablet;
    allocate sw.groundWireGuardTunnel to system.ground.tablet;

    view 'Deployment view — Survey Drone' : GeneralView {
        expose Deployment;
    }
}
