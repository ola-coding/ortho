// Logical view: what the system does, independent of what does it.
//
// A capability tree. Every function appears exactly once and has exactly one
// parent. No airframe, sensor or program is named here — realization lives in
// physical.sysml and implementation.sysml.
package Functions {

    action def OperateAircraft {

        // Energy and material functions: medium-bound, and staying that way.
        action fly {
            action generateThrust;
            action stabiliseAttitude;
            action holdPosition;
            action navigateRoute;
        }

        action senseEnvironment {
            action measureAttitude;
            action fixPosition;
            action detectObstacles;
            action rangeToGround;
        }

        action capturePayload {
            action stabiliseCamera;
            action captureVideo;
        }

        action storeEnergy {
            action holdCharge;
            action rechargeBattery;
        }

        // Control functions: the migratory ones. Attitude stabilisation was
        // mechanical gyros within living memory, and target tracking could be
        // done by a pilot's thumbs instead of by software.
        action commandAircraft {
            action acceptPilotInput;
            action displayLiveView;
            action selectTarget;
            action trackTarget;
            action avoidObstacle;
        }

        action communicate {
            action linkToController;
            action streamVideo;
            action reportTelemetry;
        }
    }
}
