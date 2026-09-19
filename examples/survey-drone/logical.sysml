// Logical view: what the system does, independent of what does it.
//
// A capability tree. Every function appears exactly once and has exactly one
// parent. No airframe, computer or program is named here — realization lives
// in physical.sysml and implementation.sysml.
package Functions {

    action def PerformAerialSurvey {

        // Energy functions: medium-bound, and staying that way.
        action operateVehicle {
            action produceThrust;
            action supplyPower;
            action provideComputing;
        }

        // Control functions: the migratory ones. Each of these was a person's
        // job before it was software — a pilot holding the lines, a navigator
        // with a chart, a spotter watching for wires.
        action guideNavigateAndControl {
            action guide;
            action navigate;
            action control;
            action avoidObstacles;
        }

        action perceive {
            action captureVideo;
            action fuseSensorData;
            action maintainWorldModel;
        }

        action communicate {
            action streamVideo;
            action exchangeTelemetry;
        }

        action plan {
            action planMission;
            action planRoute;
        }

        action interactWithOperator {
            action presentInformation;
            action acceptOperatorInput;
        }

        // Watching the other branches rather than belonging to one: health
        // monitoring and flight recording serve every function above, so their
        // lowest common ancestor is the survey itself.
        action superviseSystem {
            action monitorHealth;
            action recordFlightData;
        }
    }

    // The survey itself. The tree above says what is done; this names one
    // performance of it, so the parts that realize a function can point into
    // the tree — a `perform` chain has to start at a usage.
    action survey : PerformAerialSurvey;
}
