use super::*;

pub(crate) fn manual_occasions_repeat_and_sort_by_exact_instant(
    ledger: &(impl OccasionLedger + RunHistory),
) {
    let run = saved(ledger.begin_run(RunKind::Brief, UtcInstant { secs: 1 })).run;
    let mut first_id = None;
    for (n, at) in [
        "2026-10-01T05:00:00.123456789013Z",
        "2026-09-30T23:00:00.123456789012-06:00",
        "2026-10-01T05:00:00.123456789013Z",
    ]
    .iter()
    .enumerate()
    {
        let mut details = details();
        details.at = OccasionTime::parse(at).unwrap();
        let key = OccasionKey::manual(&details.at, &details.title).unwrap();
        let change = crate::ledger::occasion::prepare(
            ledger,
            key,
            details,
            UtcInstant { secs: n as i64 + 2 },
        )
        .unwrap();
        if n == 0 {
            first_id = Some(change.occasion.id.clone());
        }
        if n == 2 {
            assert_eq!(Some(&change.occasion.id), first_id.as_ref());
        }
        saved(write(ledger, run, change));
    }
    let rows = ledger.occasions().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].details.at.as_str(),
        "2026-09-30T23:00:00.123456789012-06:00"
    );
    assert_eq!(Some(&rows[1].id), first_id.as_ref());
}

pub(crate) fn invalid_occasion_data_is_refused_without_private_errors(
    ledger: &(impl OccasionLedger + RunHistory),
) {
    for case in 0..7 {
        let mut details = details();
        match case {
            0 => details.title.clear(),
            1 => details.title = "Private\0title".into(),
            2 => details.duration_secs = Some(u64::MAX),
            3 => details.participants = vec![details.participants[0].clone(); 257],
            4 => details.tags = vec!["Private tag".into(); 257],
            5 => {
                details.participants[0] = Participant {
                    name: String::new(),
                    email: None,
                }
            }
            _ => details.title = "P".repeat(65_537),
        }
        let result = crate::ledger::occasion::prepare(
            ledger,
            OccasionKey::dam("Private id").unwrap(),
            details,
            UtcInstant { secs: 2 },
        );
        assert!(result.is_err(), "case {case}");
        assert!(!format!("{result:?}").contains("Private"));
    }
    let details = details();
    let key = OccasionKey::manual(&details.at, "Different title").unwrap();
    assert!(
        crate::ledger::occasion::prepare(ledger, key, details, UtcInstant { secs: 2 }).is_err()
    );
    assert!(ledger.occasions().unwrap().is_empty());
    assert!(ledger.pending_exports().unwrap().is_empty());
}
