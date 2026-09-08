// Deployment view: which compute node runs which software.
//
// This file declares nothing of its own. Its whole content is the mapping
// between the parts in implementation.sysml and the parts in physical.sysml,
// which is why it must be rendered together with both.
package Deployment {

    import Software::*;
    import Hardware::*;

    // Four programs share the airborne SoC.
    allocate Software::sw.flightControl to Hardware::Aircraft::fc;
    allocate Software::sw.navigation to Hardware::Aircraft::fc;
    allocate Software::sw.perception to Hardware::Aircraft::fc;
    allocate Software::sw.targetTracking to Hardware::Aircraft::fc;

    // The payload and the radio each carry their own.
    allocate Software::sw.cameraCtrl to Hardware::Aircraft::camera;
    allocate Software::sw.telemetry to Hardware::Aircraft::radio;

    // And the ground application runs on the handset.
    allocate Software::sw.pilotApp to Hardware::AuvSystem::controller;
}
