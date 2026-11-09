use super::*;
#[test]
fn selection_uses_earliest_incomplete_stage_and_never_retries_success_or_expiration() {
    use StageState::*;
    for transcribe in [Pending, Failed, Disabled, Succeeded, Expired] {
        for note in [Pending, Failed, Disabled, Succeeded, Expired] {
            let states = StageStates {
                transcribe,
                note,
                synthesis: Pending,
            };
            let expected = match (transcribe, note) {
                (Pending | Failed | Disabled, _) => Some(NextStage::Transcribe),
                (_, Pending | Failed | Disabled) => Some(NextStage::Note),
                _ => None,
            };
            assert_eq!(next(states), expected, "{states:?}");
        }
    }
}
