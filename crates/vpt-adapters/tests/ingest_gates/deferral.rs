use super::*;

#[test]
fn deferral_threshold_commits_seen_work_and_one_typed_event_together() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("private-voice.m4a", b"private media canary");
    let mut parts = fixture.parts();
    parts.settings.deferral_page_threshold = 2;

    let first = parts.ingest().run(&Mode::default()).unwrap();
    let second = parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(first.deferred[0].reason, DeferralReason::InvalidContainer);
    assert_eq!(second.deferred[0].reason, DeferralReason::InvalidContainer);
    let row = parts.ledger.seen(&source).unwrap().unwrap();
    assert_eq!(row.deferral_count, 2);
    assert_eq!(
        row.deferred_size,
        Some(std::fs::metadata(&source).unwrap().len())
    );
    let deferred: Vec<_> = parts
        .sink
        .receipts
        .iter()
        .filter(|receipt| {
            receipt.records.iter().any(|record| {
                matches!(
                    record.fact,
                    HistoryFact::WorkStep {
                        outcome: WorkOutcome::Deferred,
                        ..
                    }
                )
            })
        })
        .collect();
    assert_eq!(deferred.len(), 2);
    assert_eq!(deferred[0].records.len(), 1);
    assert_eq!(deferred[1].records.len(), 2);
    assert_eq!(deferred[1].completed.len(), 0);
    let event = deferred[1]
        .records
        .iter()
        .find_map(|record| match &record.fact {
            HistoryFact::Event(event) => Some(event),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        (event.kind, event.state),
        (EventKind::Deferred, EventState::NeedsAttention)
    );
    assert_eq!(
        event.detail,
        EventDetail::Deferred {
            reason: DeferralReason::InvalidContainer,
            count: 2
        }
    );
    assert_eq!(event.paths.len(), 1);
    assert_eq!(event.paths[0].key, EventPathKey::Source);
    assert_eq!(event.paths[0].path, source);
}

#[test]
fn above_threshold_does_not_repeat_deferred_event() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("partial.m4a", b"partial");
    let mut parts = fixture.parts();
    parts.settings.deferral_page_threshold = 1;

    for _ in 0..3 {
        assert_eq!(
            parts.ingest().run(&Mode::default()).unwrap().deferred.len(),
            1
        );
    }

    assert_eq!(
        parts.ledger.seen(&source).unwrap().unwrap().deferral_count,
        3
    );
    assert_eq!(
        parts
            .ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .filter(|record| matches!(record.fact, HistoryFact::Event(_)))
            .count(),
        1
    );
}

#[test]
fn deferred_observation_preserves_first_seen_and_resets_source_gone() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("observed.m4a", b"partial");
    let mut parts = fixture.parts();
    parts.ingest().run(&Mode::default()).unwrap();
    let mut previous = parts.ledger.seen(&source).unwrap().unwrap();
    previous.source_gone_at = Some(parts.clock.now);
    let first_seen = previous.first_seen;
    parts.ledger.record_seen(&previous).unwrap();
    parts.clock.now.secs += 1;
    let probe = GateRecorder::new(&fixture.recordings);
    probe.extra_flags.set(0x20);

    probed(&mut parts, &probe).unwrap();

    let observed = parts.ledger.seen(&source).unwrap().unwrap();
    assert_eq!(observed.first_seen, first_seen);
    assert_eq!(observed.last_seen, parts.clock.now);
    assert_eq!(observed.source_gone_at, None);
    assert_eq!(observed.flags & 0x20, 0x20);
    assert_eq!(observed.deferral_count, 2);
}

#[test]
fn deferral_count_saturates_without_repeating_threshold_event() {
    let fixture = Fixture::new();
    let source = fixture.add_recording("saturated.m4a", b"partial");
    let mut parts = fixture.parts();
    parts.ingest().run(&Mode::default()).unwrap();
    let mut previous = parts.ledger.seen(&source).unwrap().unwrap();
    previous.deferral_count = u32::MAX;
    parts.ledger.record_seen(&previous).unwrap();
    parts.settings.deferral_page_threshold = 2;

    parts.ingest().run(&Mode::default()).unwrap();

    assert_eq!(
        parts.ledger.seen(&source).unwrap().unwrap().deferral_count,
        u32::MAX
    );
    assert_eq!(
        parts
            .ledger
            .recovery_view()
            .unwrap()
            .records
            .iter()
            .filter(|record| matches!(record.fact, HistoryFact::Event(_)))
            .count(),
        0
    );
}
