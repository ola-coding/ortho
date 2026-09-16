// Deployment view: which computer runs which software.
//
// This file declares nothing of its own. Its whole content is the mapping
// between the parts in implementation.sysml and the parts in physical.sysml,
// which is why it must be rendered together with both. Every module named in
// implementation.sysml is allocated here — a computer's own platform included,
// since a kernel runs on a processor like anything else.
package Deployment {

    import Software::*;
    import Hardware::*;

    // The flight firmware has the control computer to itself: the control
    // libraries, the drivers that feed them, and the kernel underneath.
    allocate Software::sw.mission to Hardware::SurveyDrone::fc;
    allocate Software::sw.estimator to Hardware::SurveyDrone::fc;
    allocate Software::sw.control to Hardware::SurveyDrone::fc;
    allocate Software::sw.sensors to Hardware::SurveyDrone::fc;
    allocate Software::sw.mavlink to Hardware::SurveyDrone::fc;
    allocate Software::sw.hal to Hardware::SurveyDrone::fc;
    allocate Software::sw.rtos to Hardware::SurveyDrone::fc;

    // Every onboard container shares the GPU computer, with the middleware
    // and the Linux platform they stand on.
    allocate Software::sw.planner to Hardware::SurveyDrone::companion;
    allocate Software::sw.avoidance to Hardware::SurveyDrone::companion;
    allocate Software::sw.video to Hardware::SurveyDrone::companion;
    allocate Software::sw.fusion to Hardware::SurveyDrone::companion;
    allocate Software::sw.world to Hardware::SurveyDrone::companion;
    allocate Software::sw.diagnostics to Hardware::SurveyDrone::companion;
    allocate Software::sw.logger to Hardware::SurveyDrone::companion;
    allocate Software::sw.zenoh to Hardware::SurveyDrone::companion;
    allocate Software::sw.bridge to Hardware::SurveyDrone::companion;
    allocate Software::sw.gstreamer to Hardware::SurveyDrone::companion;
    allocate Software::sw.airTunnel to Hardware::SurveyDrone::companion;
    allocate Software::sw.runtime to Hardware::SurveyDrone::companion;
    allocate Software::sw.ota to Hardware::SurveyDrone::companion;
    allocate Software::sw.linux to Hardware::SurveyDrone::companion;
    allocate Software::sw.secureBoot to Hardware::SurveyDrone::companion;

    // The operator's app runs on the tablet, behind its own end of the tunnel.
    allocate Software::sw.app to Hardware::GroundStation::tablet;
    allocate Software::sw.groundTunnel to Hardware::GroundStation::tablet;
}
