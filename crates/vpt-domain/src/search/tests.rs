use super::*;

#[test]
fn a_transcript_line_reads_back_as_the_spoken_text_the_writer_escaped() {
    let spoken = "a&b <c> [d] \\ `e` *f* _g_ {h} #i |j ~k 5:30 @l %m =n ^o $p www.q";
    let line = format!("[75:10] {}", crate::note::escape_text(spoken).unwrap());
    let read: Vec<_> = moments(NoteStage::Transcript, &line).collect();
    assert_eq!(
        read,
        [Moment {
            seconds: Some(4510),
            quote: spoken.into()
        }]
    );
}

#[test]
fn an_analysis_claim_takes_its_first_source_and_drops_annotations() {
    let content = "## Summary\n\n- Ship it &#58; now [01:02-01:05] [02:00-02:10]\n- Unsourced [unsourced]\n\n## Actions\n";
    let read: Vec<_> = moments(NoteStage::Analysis, content).collect();
    assert_eq!(
        read,
        [
            Moment {
                seconds: Some(62),
                quote: "Ship it : now".into()
            },
            Moment {
                seconds: None,
                quote: "Unsourced".into()
            },
        ]
    );
}

#[test]
fn a_query_needs_every_word_in_one_line_in_any_case() {
    let query = Query::new(&["Budget".into(), "fri".into()]).unwrap();
    assert!(query.matches("the BUDGET moves to Friday"));
    assert!(!query.matches("the budget moves"));
    assert!(Query::new(&[]).is_none());
}
