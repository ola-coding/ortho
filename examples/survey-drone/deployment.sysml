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

    // The flight firmware has the control computer to itself: the control
    // libraries, the drivers that feed them, and the kernel underneath.
    allocate Software::sw.mission to Hardware::system.drone.fc;
    allocate Software::sw.estimator to Hardware::system.drone.fc;
    allocate Software::sw.control to Hardware::system.drone.fc;
    allocate Software::sw.sensors to Hardware::system.drone.fc;
    allocate Software::sw.mavlink to Hardware::system.drone.fc;
    allocate Software::sw.hal to Hardware::system.drone.fc;
    allocate Software::sw.rtos to Hardware::system.drone.fc;

    // Every onboard container shares the GPU computer, with the middleware
    // and the Linux platform they stand on.
    allocate Software::sw.planner to Hardware::system.drone.companion;
    allocate Software::sw.avoidance to Hardware::system.drone.companion;
    allocate Software::sw.video to Hardware::system.drone.companion;
    allocate Software::sw.fusion to Hardware::system.drone.companion;
    allocate Software::sw.world to Hardware::system.drone.companion;
    allocate Software::sw.diagnostics to Hardware::system.drone.companion;
    allocate Software::sw.logger to Hardware::system.drone.companion;
    allocate Software::sw.zenoh to Hardware::system.drone.companion;
    allocate Software::sw.bridge to Hardware::system.drone.companion;
    allocate Software::sw.gstreamer to Hardware::system.drone.companion;
    allocate Software::sw.airTunnel to Hardware::system.drone.companion;
    allocate Software::sw.runtime to Hardware::system.drone.companion;
    allocate Software::sw.ota to Hardware::system.drone.companion;
    allocate Software::sw.linux to Hardware::system.drone.companion;
    allocate Software::sw.secureBoot to Hardware::system.drone.companion;

    // The operator's app runs on the tablet, behind its own end of the tunnel.
    allocate Software::sw.app to Hardware::system.ground.tablet;
    allocate Software::sw.groundTunnel to Hardware::system.ground.tablet;
}
