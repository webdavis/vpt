use super::*;

#[test]
fn relation_defaults_and_overrides_reach_runtime_settings() {
    let defaults = settings("").unwrap().relations;
    assert_eq!(defaults.session_gap_minutes, 60);
    assert_eq!(defaults.max_suggested, 5);
    let overridden = settings("[relations]\nsession_gap_minutes=15\nmax_suggested=3\n")
        .unwrap()
        .relations;
    assert_eq!(overridden.session_gap_minutes, 15);
    assert_eq!(overridden.max_suggested, 3);
    let disabled = settings("[relations]\nsession_gap_minutes=0\nmax_suggested=0\n")
        .unwrap()
        .relations;
    assert_eq!(disabled.session_gap_minutes, 0);
    assert_eq!(disabled.max_suggested, 0);
}
