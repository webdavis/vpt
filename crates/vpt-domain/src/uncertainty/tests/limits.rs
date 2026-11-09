use super::*;

#[test]
fn invalid_thresholds_and_nonfinite_confidence_are_refused() {
    for floor in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert_eq!(
            classify(
                &transcript("ordinary"),
                None,
                Options {
                    confidence_floor: floor,
                    ..Options::default()
                },
                &[]
            ),
            Err(ClassificationError::InvalidOptions)
        );
    }
    for ratio in [0.0, -1.0, f64::NAN, 1.1] {
        assert_eq!(
            classify(
                &transcript("ordinary"),
                None,
                Options {
                    max_divergence_ratio: ratio,
                    ..Options::default()
                },
                &[]
            ),
            Err(ClassificationError::InvalidOptions)
        );
    }
    let mut input = transcript("ordinary");
    for confidence in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        input.segments[0].words[0].confidence = Some(confidence);
        assert_eq!(
            classify(&input, None, Options::default(), &[]),
            Err(ClassificationError::InvalidConfidence)
        );
    }
}

#[test]
fn single_engine_input_and_known_terms_are_bounded_too() {
    let input = transcript(&vec!["same"; 65_537].join(" "));
    assert_eq!(
        classify(&input, None, Options::default(), &[]),
        Err(ClassificationError::CapacityExceeded)
    );
    let huge = "x".repeat(16 * 1024 * 1024 + 1);
    assert_eq!(
        classify(&transcript(&huge), None, Options::default(), &[]),
        Err(ClassificationError::CapacityExceeded)
    );
    assert_eq!(
        classify(&transcript("ordinary"), None, Options::default(), &[&huge]),
        Err(ClassificationError::CapacityExceeded)
    );
}
