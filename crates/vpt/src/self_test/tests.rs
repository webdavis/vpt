use super::plan::{Part, checks};
use crate::report::Row;
use crate::style::Verdict;
use std::cell::Cell;

fn ran() -> Vec<Row> {
    vec![Row::ok("one", "ran")]
}

#[test]
fn a_named_part_with_nothing_to_test_is_refused_before_anything_runs() {
    let called = Cell::new(false);
    let run = || {
        called.set(true);
        ran()
    };
    let parts = vec![
        Part {
            name: "ready",
            blurb: "",
            run: Some(&run),
        },
        Part {
            name: "absent",
            blurb: "",
            run: None,
        },
    ];
    let error = checks(parts, true).err().unwrap();
    assert_eq!(error.kind.exit_code(), 2);
    assert!(error.message.contains("absent"), "{}", error.message);
    assert!(!error.message.contains("ready"), "{}", error.message);
    assert!(!called.get());
}

#[test]
fn a_bare_run_marks_an_unconfigured_part_not_run_and_runs_the_rest() {
    let parts = vec![
        Part {
            name: "ready",
            blurb: "",
            run: Some(&ran),
        },
        Part {
            name: "absent",
            blurb: "",
            run: None,
        },
    ];
    let checks = checks(parts, false).unwrap();
    let names: Vec<&str> = checks.iter().map(|check| check.name).collect();
    assert_eq!(names, ["ready", "absent"]);
    assert_eq!(checks[0].rows[0].detail, "ran");
    assert_eq!(checks[1].rows[0].verdict, Verdict::NotRun);
}

#[test]
fn a_bare_run_that_tests_nothing_ends_with_an_issue() {
    let parts = vec![Part {
        name: "absent",
        blurb: "",
        run: None,
    }];
    let checks = checks(parts, false).unwrap();
    let last = checks.last().unwrap();
    assert_eq!(last.name, "run");
    assert_eq!(last.rows[0].verdict, Verdict::Issue);
    assert!(last.rows[0].detail.contains("nothing was tested"));
}
