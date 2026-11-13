// Use case view: what an operator asks the system for.
package SystemUseCases {
    doc /* One need, in the operator's words: survey a ground area from the air,
         * plan it, and follow its progress while it flies. */
    private import StandardViewDefinitions::*;

    // The subject is the system as a black box: a part def of its own, naming
    // neither the drone, the tablet nor the radio. physical.sysml opens it up.
    part def SurveyDroneSystem;
    part def Operator;

    use case def SurveyTask {
        subject system : SurveyDroneSystem;
        actor operator : Operator;
    }

    use case surveyArea : SurveyTask {
        doc /* Surveying an area always means planning it first and following it
             * as it flies. */
        include planSurvey;
        include followProgress;
    }

    use case planSurvey : SurveyTask {
        doc /* Also on its own: an operator can plan tomorrow's survey. */
    }

    use case followProgress : SurveyTask {
        doc /* Also on its own: an operator can check on a survey under way. */
    }

    view 'Use case view — Survey Drone' : GeneralView {
        expose SystemUseCases;
    }
}
