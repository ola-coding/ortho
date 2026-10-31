// Use case view: what an operator asks the system for.
//
// One need, in the operator's words: survey a ground area from the air, plan
// it, and follow its progress while it flies. The subject is the system as a
// black box: a part def of its own, naming neither the drone, the tablet nor
// the radio.
package SystemUseCases {
    private import StandardViewDefinitions::*;

    part def Operator;
    part def SurveyDroneSystem;

    use case def SurveyTask {
        subject system : SurveyDroneSystem;
        actor operator : Operator;
    }

    // Surveying an area always means planning it first and following it as
    // it flies. Each also stands on its own: an operator can plan tomorrow's
    // survey, or check on one already under way.
    use case surveyArea : SurveyTask {
        include planSurvey;
        include followProgress;
    }

    use case planSurvey : SurveyTask;

    use case followProgress : SurveyTask;

    view 'Use case view — Survey Drone' : GeneralView {
        expose SystemUseCases;
    }
}
