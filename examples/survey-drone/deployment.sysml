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
    allocate sw.mission to system.drone.fc;
    allocate sw.estimator to system.drone.fc;
    allocate sw.control to system.drone.fc;
    allocate sw.sensors to system.drone.fc;
    allocate sw.mavlink to system.drone.fc;
    allocate sw.hal to system.drone.fc;
    allocate sw.rtos to system.drone.fc;

    // Every onboard container shares the GPU computer, with the middleware
    // and the Linux platform they stand on.
    allocate sw.planner to system.drone.companion;
    allocate sw.avoidance to system.drone.companion;
    allocate sw.video to system.drone.companion;
    allocate sw.fusion to system.drone.companion;
    allocate sw.world to system.drone.companion;
    allocate sw.diagnostics to system.drone.companion;
    allocate sw.logger to system.drone.companion;
    allocate sw.zenoh to system.drone.companion;
    allocate sw.bridge to system.drone.companion;
    allocate sw.gstreamer to system.drone.companion;
    allocate sw.airTunnel to system.drone.companion;
    allocate sw.runtime to system.drone.companion;
    allocate sw.ota to system.drone.companion;
    allocate sw.linux to system.drone.companion;
    allocate sw.secureBoot to system.drone.companion;

    // The operator's app runs on the tablet, behind its own end of the tunnel.
    allocate sw.app to system.ground.tablet;
    allocate sw.groundTunnel to system.ground.tablet;

    view 'Deployment view — Survey Drone' : GeneralView {
        expose Deployment;
    }
}
