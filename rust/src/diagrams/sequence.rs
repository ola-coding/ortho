/// A lifeline, headed by its part as declared: `uiApp : UiApp`, or a person's `user`.
pub struct SequenceLifeline {
    pub id: String,
    pub label: String,
}

pub struct SequenceMessage {
    pub source_id: String,
    pub target_id: String,
    pub label: String,
}

pub struct SequenceModel {
    pub lifelines: Vec<SequenceLifeline>,
    pub messages: Vec<SequenceMessage>,
}
