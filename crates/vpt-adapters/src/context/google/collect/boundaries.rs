use super::{tests::*, *};
use std::cell::Cell;

#[test]
fn shared_deadline_includes_refresh_and_all_pages_and_discards_partial_results() {
    let clock = Cell::new(Instant::now());
    let mut budgets = Vec::new();
    let snapshot = run(
        &credentials(),
        &request(&["cal".into()]),
        |call, budget| {
            budgets.push(budget.as_secs());
            clock.set(clock.get() + Duration::from_secs(10));
            Ok(match call {
                Request::Refresh(_) => token(token::SCOPE),
                _ => page("event", 1000, Some("next")),
            })
        },
        || clock.get(),
        || false,
    )
    .unwrap();
    assert_eq!(budgets, vec![30, 20, 10]);
    assert_eq!(
        snapshot.status.calendar.detail(),
        Some(Failure::Deadline.detail())
    );
    assert!(snapshot.occasions.is_empty());
}

#[test]
fn interruption_takes_precedence_over_a_late_transport_reply() {
    let clock = Cell::new(Instant::now());
    let stopped = Cell::new(false);
    let result = run(
        &credentials(),
        &request(&["cal".into()]),
        |_, _| {
            clock.set(clock.get() + Duration::from_secs(31));
            stopped.set(true);
            Ok(token(token::SCOPE))
        },
        || clock.get(),
        || stopped.get(),
    );
    assert_eq!(result, Err(ContextError::Interrupted));
}

#[test]
fn missing_selection_or_credentials_refuse_before_network_access() {
    let result = run(
        &credentials(),
        &request(&[]),
        |_, _| panic!("unexpected request"),
        Instant::now,
        || false,
    );
    assert_eq!(
        result,
        Err(ContextConfigError {
            key: "context.calendars"
        }
        .into())
    );
    for (field, key) in [
        (0, "context.google.client_id"),
        (1, "context.google.client_secret"),
        (2, "context.google.refresh_token"),
    ] {
        let mut values = credentials();
        match field {
            0 => values.client_id.clear(),
            1 => values.client_secret.clear(),
            _ => values.refresh_token.clear(),
        }
        let result = run(
            &values,
            &request(&["primary".into()]),
            |_, _| panic!("unexpected request"),
            Instant::now,
            || false,
        );
        assert_eq!(result, Err(ContextConfigError { key }.into()));
    }
}

#[test]
fn failure_in_a_later_calendar_or_a_repeated_cursor_invalidates_all_discovery() {
    for later_calendar in [false, true] {
        let mut pages = 0;
        let snapshot = run(
            &credentials(),
            &request(&["one".into(), "two".into()]),
            |call, _| match call {
                Request::Refresh(_) => Ok(token(token::SCOPE)),
                Request::Events { calendar, .. } => {
                    pages += 1;
                    if calendar == "two" {
                        Err(Failure::HttpStatus)
                    } else {
                        Ok(page(
                            "event",
                            1000,
                            if later_calendar {
                                None
                            } else {
                                Some("repeated")
                            },
                        ))
                    }
                }
            },
            Instant::now,
            || false,
        )
        .unwrap();
        assert_eq!(pages, 2);
        assert!(matches!(
            snapshot.status.calendar,
            ContextStatus::Failed { .. }
        ));
        assert!(snapshot.occasions.is_empty());
    }
}

#[test]
fn exactly_ten_complete_pages_are_accepted_and_duplicate_selections_are_not_refetched() {
    let mut pages = 0;
    let snapshot = run(
        &credentials(),
        &request(&["one".into(), "one".into()]),
        |call, _| match call {
            Request::Refresh(_) => Ok(token(token::SCOPE)),
            _ => {
                pages += 1;
                let next = format!("next-{pages}");
                Ok(page(
                    &format!("event-{pages}"),
                    1000,
                    if pages == 10 { None } else { Some(&next) },
                ))
            }
        },
        Instant::now,
        || false,
    )
    .unwrap();
    assert_eq!(pages, 10);
    assert_eq!(snapshot.occasions.len(), 10);
    assert_eq!(snapshot.status.calendar, ContextStatus::Present);
}
