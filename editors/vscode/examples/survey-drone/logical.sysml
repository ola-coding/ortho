// Logical view: what the system does, independent of what does it.
package Functions {
    doc /* A capability tree. Every function appears exactly once and has exactly
         * one parent. No airframe, computer or program is named here. */
    private import StandardViewDefinitions::*;
    private import Views::*;
    private import ModelingMetadata::*;

    // Branches in the order a survey meets them: the operator plans it, the
    // vehicle flies it, and its results come home.
    action def PerformAerialSurvey {

        action interactWithOperator {
            action planMission;
            action presentProgress;
        }

        // Control functions: the migratory ones. Each of these was a person's
        // job before it was software: a navigator with a chart, a pilot holding
        // the lines, a spotter watching for wires.
        action guideNavigateAndControl {
            action planRoute;
            action guide;
            action navigate;
            action control;
            action avoidObstacles;
        }

        action perceive {
            action captureVideo;
            action maintainWorldModel;
        }

        action communicate {
            action streamVideo;
            action exchangeTelemetry;
        }

        // Energy functions: medium-bound, and staying that way.
        action operateVehicle {
            action supplyPower;
            action produceThrust;
        }

        // Health monitoring and flight recording serve every branch above, so
        // their lowest common ancestor is the survey itself.
        action superviseSystem;
    }

    // One performance of the tree. A `perform` chain has to start at a usage,
    // so the parts that realize a function point into this one.
    action survey : PerformAerialSurvey;

    // What the operator asks for, in more detail.
    #refinement dependency PerformAerialSurvey to SystemUseCases::surveyArea;

    view 'Logical view — Survey Drone' : GeneralView {
        expose PerformAerialSurvey;
        render asTreeDiagram;
    }
}
