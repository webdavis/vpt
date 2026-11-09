use super::*;

fn finding(evidence: &str, text: &str) -> Option<Finding> {
    let words = words(&[(evidence, 0.0, 1.0)]);
    Grounder::new(words.iter(), &[], 2.0, 0)
        .unwrap()
        .check(&claim(text, &["00:00-00:01"]))
        .unwrap()
        .finding
}

#[test]
fn whole_names_and_numbers_are_not_substrings() {
    for (evidence, text) in [
        ("Malice", "Alice"),
        ("100", "10"),
        ("$100", "$10"),
        ("foobob", "Bob"),
    ] {
        assert!(
            matches!(finding(evidence, text), Some(Finding::Unsupported(_))),
            "{evidence}: {text}"
        );
    }
    for (evidence, text) in [
        ("alice", "Alice"),
        ("STRASSE", "Straße"),
        ("Ａｌｉｃｅ ４２", "Alice 42"),
        ("$10", "10"),
        ("code42", "42"),
        ("42", "code42"),
    ] {
        assert_eq!(finding(evidence, text), None, "{evidence}: {text}");
    }
}

#[test]
fn currency_dates_signs_and_separators_keep_their_meaning() {
    for (evidence, text) in [
        ("$1,000", "$1.000"),
        ("€1,000", "$1,000"),
        ("1000", "$1,000"),
        ("2026-09-29", "2026/09/29"),
        ("20260929", "2026-09-29"),
        ("10", "-10"),
        ("10", "−10"),
        ("1 and 4", "¼"),
        ("10", "10%"),
        ("1.25", "1,25"),
        ("5", ".5"),
        ("$10", "-$10"),
        ("10 dollars and 20 euros", "20 dollars"),
        ("10 $ and 20 €", "20 $"),
        ("10 dollars", "10 Euros"),
        ("twenty dollars", "ten dollars"),
        ("one hundred dollars and twenty euros", "one hundred euros"),
        ("USD twenty", "USD ten"),
        ("May 11 and June 10", "May 10"),
        ("May 10, 2025 and 2026", "May 10, 2026"),
        ("10 June and 11 May", "10 May"),
        ("USD 20 and EUR 10", "USD 10"),
    ] {
        assert!(
            matches!(finding(evidence, text), Some(Finding::Unsupported(_))),
            "{evidence}: {text}"
        );
    }
    for text in [
        "$1,000",
        "2026-09-29",
        "-10",
        "+10",
        "10%",
        "1.25",
        "May 10, 2026",
        "10 May 2026",
        "USD 10",
    ] {
        assert_eq!(finding(text, &format!("({text}).")), None, "{text}");
    }
    for (evidence, text) in [
        ("$ 10", "$10"),
        ("$10", "$ 10"),
        ("May 10, 2026", "May 10"),
        ("May 10 2026", "May 10, 2026"),
        ("usd 10", "USD 10"),
        ("10 dollars", "10 dollars"),
        ("10€", "10 €"),
        ("10 euros", "10 Euros"),
        ("10 $ and 20 €", "10 $"),
        ("ten dollars", "ten dollars"),
        ("One hundred dollars", "one hundred dollars"),
        ("USD one hundred", "USD one hundred"),
    ] {
        assert_eq!(finding(evidence, text), None, "{evidence}: {text}");
    }
}

#[test]
fn unsupported_facts_are_reported_in_claim_order_without_semantic_paraphrase_checks() {
    assert_eq!(
        finding("alice 10", "Bob paid 12"),
        Some(Finding::Unsupported("Bob".into()))
    );
    assert_eq!(
        finding("nothing about payments", "a payment was approved"),
        None
    );
    assert_eq!(finding("Alice,Bob; Carol!", "Alice Bob Carol"), None);
}

#[test]
fn disjoint_citations_do_not_invent_an_adjacent_date() {
    let words = words(&[
        ("May", 0.0, 1.0),
        ("11", 1.0, 2.0),
        ("June", 8.0, 9.0),
        ("10", 9.0, 10.0),
    ]);
    let mut checker = Grounder::new(words.iter(), &[], 10.0, 0).unwrap();
    assert!(matches!(
        checker
            .check(&claim("May 10", &["00:00-00:01", "00:09-00:10"]))
            .unwrap()
            .finding,
        Some(Finding::Unsupported(_))
    ));
}

#[test]
fn adjacent_citations_support_a_date_across_their_boundary() {
    let words = words(&[("May", 0.0, 1.0), ("10", 1.0, 2.0)]);
    let mut checker = Grounder::new(words.iter(), &[], 2.0, 0).unwrap();
    assert_eq!(
        checker
            .check(&claim("May 10", &["00:01-00:02", "00:00-00:01"]))
            .unwrap()
            .finding,
        None
    );
}
