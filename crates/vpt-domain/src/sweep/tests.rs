use super::*;

const LIMITS: SweepLimits = SweepLimits {
    max_audio_bytes: 2_147_483_648,
    quiet_period_secs: 30,
};
const UF_HIDDEN: u32 = 0x0000_8000;

fn candidate(size: u64, secs: i64, nanos: u32, flags: u32) -> CandidateFacts {
    CandidateFacts {
        size,
        mtime: FileTime { secs, nanos },
        flags,
    }
}

#[test]
fn dataless_precedes_unchanged_and_other_flags_do_not_defer() {
    let seen = SeenFacts {
        size: 10,
        mtime: FileTime { secs: 5, nanos: 0 },
        ingested: true,
        deferred_size: None,
    };
    assert_eq!(
        pre_open(&candidate(10, 5, 0, SF_DATALESS), Some(&seen)),
        PreOpen::Dataless
    );
    assert_eq!(
        pre_open(&candidate(10, 5, 0, SF_DATALESS | UF_HIDDEN), None),
        PreOpen::Dataless
    );
    assert_eq!(
        pre_open(&candidate(10, 5, 0, UF_HIDDEN), None),
        PreOpen::Open
    );
}

#[test]
fn unchanged_ingested_facts_skip_and_changes_open() {
    let seen = SeenFacts {
        size: 10,
        mtime: FileTime { secs: 5, nanos: 1 },
        ingested: true,
        deferred_size: None,
    };
    assert_eq!(
        pre_open(&candidate(10, 5, 1, 0), Some(&seen)),
        PreOpen::Unchanged
    );
    assert_eq!(
        pre_open(&candidate(11, 5, 1, 0), Some(&seen)),
        PreOpen::Open
    );
    assert_eq!(
        pre_open(&candidate(10, 6, 1, 0), Some(&seen)),
        PreOpen::Open
    );
    assert_eq!(
        pre_open(&candidate(10, 5, 2, 0), Some(&seen)),
        PreOpen::Open
    );
    let deferred = SeenFacts {
        ingested: false,
        ..seen
    };
    assert_eq!(
        pre_open(&candidate(10, 5, 1, 0), Some(&deferred)),
        PreOpen::Open
    );
}

#[test]
fn size_gate_accepts_the_limit_and_defers_one_byte_more() {
    assert_eq!(size_gate(2_147_483_648, &LIMITS), Ok(()));
    assert_eq!(
        size_gate(2_147_483_649, &LIMITS),
        Err(DeferralReason::AudioTooLarge)
    );
}

#[test]
fn rest_gate_requires_the_whole_quiet_period_to_the_nanosecond() {
    let now = UtcInstant { secs: 1_000 };
    assert_eq!(
        rest_gate(
            FileTime {
                secs: 971,
                nanos: 0
            },
            10,
            now,
            None,
            &LIMITS
        ),
        Err(DeferralReason::NotAtRest)
    );
    assert_eq!(
        rest_gate(
            FileTime {
                secs: 970,
                nanos: 1
            },
            10,
            now,
            None,
            &LIMITS
        ),
        Err(DeferralReason::NotAtRest)
    );
    assert_eq!(
        rest_gate(
            FileTime {
                secs: 970,
                nanos: 0
            },
            10,
            now,
            None,
            &LIMITS
        ),
        Ok(())
    );
}

#[test]
fn rest_gate_uses_the_full_width_of_the_quiet_period() {
    let limits = SweepLimits {
        max_audio_bytes: 1,
        quiet_period_secs: u64::MAX,
    };
    assert_eq!(
        rest_gate(
            FileTime { secs: 0, nanos: 0 },
            10,
            UtcInstant { secs: i64::MAX },
            None,
            &limits,
        ),
        Err(DeferralReason::NotAtRest)
    );
    assert_eq!(
        rest_gate(
            FileTime {
                secs: i64::MIN,
                nanos: 1
            },
            10,
            UtcInstant { secs: i64::MAX },
            None,
            &limits,
        ),
        Err(DeferralReason::NotAtRest)
    );
    assert_eq!(
        rest_gate(
            FileTime {
                secs: i64::MIN,
                nanos: 0
            },
            10,
            UtcInstant { secs: i64::MAX },
            None,
            &limits,
        ),
        Ok(())
    );
}

#[test]
fn changed_size_since_deferral_is_not_at_rest() {
    let now = UtcInstant { secs: 1_000 };
    let seen = SeenFacts {
        size: 9,
        mtime: FileTime { secs: 1, nanos: 0 },
        ingested: false,
        deferred_size: Some(9),
    };
    assert_eq!(
        rest_gate(
            FileTime { secs: 1, nanos: 0 },
            10,
            now,
            Some(&seen),
            &LIMITS
        ),
        Err(DeferralReason::NotAtRest)
    );
    assert_eq!(
        rest_gate(FileTime { secs: 1, nanos: 0 }, 9, now, Some(&seen), &LIMITS),
        Ok(())
    );
}

#[test]
fn deferral_reasons_use_persisted_spellings() {
    for (reason, spelling) in [
        (DeferralReason::Dataless, "dataless"),
        (DeferralReason::AudioTooLarge, "audio_too_large"),
        (DeferralReason::InvalidContainer, "invalid_container"),
        (DeferralReason::NotAtRest, "not_at_rest"),
        (DeferralReason::ChangedDuringRead, "changed_during_read"),
    ] {
        assert_eq!(reason.as_str(), spelling);
        assert_eq!(DeferralReason::parse(spelling), Some(reason));
        assert_eq!(DeferralReason::parse(reason.as_str()), Some(reason));
    }
    assert_eq!(DeferralReason::parse("busy"), None);
}
