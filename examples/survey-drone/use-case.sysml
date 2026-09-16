// Use case view: what an operator asks the system for.
//
// One need, in the operator's words: survey a ground area from the air, plan
// it, and follow its progress while it flies. The subject is untyped: naming
// the drone, the tablet or the radio would be realization detail, and this
// view carries none.
package UseCases {

    part def Operator;

    use case def SurveyTask {
        subject SurveyDroneSystem;
        actor operator : Operator;
    }

    // Surveying an area always means planning it first and following it as
    // it flies. Each also stands on its own: an operator can plan tomorrow's
    // survey, or check on one already under way.
    use case surveyArea : SurveyTask {
        include use case planSurvey;
        include use case followProgress;
    }

    use case planSurvey : SurveyTask;

    use case followProgress : SurveyTask;
}
