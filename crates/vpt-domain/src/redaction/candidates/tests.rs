use super::*;
#[test]
fn candidates_keep_each_capitalized_survivor_with_unicode_and_sentence_boundaries() {
    let text =
        "Start with Élodie and O’Neil’s plan. Next Café? Yes NASA! Then ǅuro。After Zoë\nwith Zoë.";
    let found = candidates(text, &[]).unwrap();
    assert_eq!(
        found.iter().map(|c| c.token.as_str()).collect::<Vec<_>>(),
        ["Élodie", "O’Neil’s", "Café", "NASA", "ǅuro", "Zoë", "Zoë"]
    );
    for candidate in &found {
        assert_eq!(&text[candidate.bytes.clone()], candidate.token);
        assert!(!format!("{candidate:?}").contains(&candidate.token));
    }
}
#[test]
fn generated_masks_do_not_produce_candidates_or_introduce_sentence_boundaries() {
    let mut redactor = Redactor::new(
        &["Alice"],
        &[],
        Options {
            mask: "[Hidden! {n}]".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let output = redactor
        .apply_with_spans("Alice Bob and [Hidden! 1] Carol", &[])
        .unwrap();
    assert_eq!(output.masks.len(), 1);
    assert_eq!(&output.text[output.masks[0].clone()], "[Hidden! 1]");
    assert_eq!(
        candidates(&output.text, &output.masks)
            .unwrap()
            .iter()
            .map(|c| c.token.as_str())
            .collect::<Vec<_>>(),
        ["Bob", "Hidden", "Carol"]
    );
}
#[test]
fn candidate_collection_refuses_oversize_text_and_invalid_mask_offsets_privately() {
    assert_eq!(
        candidates(&"X".repeat(MAX_TEXT + 1), &[]),
        Err(Error::Capacity)
    );
    for (start, end) in [(1, 2), (0, 99), (2, 1)] {
        let range = start..end;
        assert_eq!(
            candidates("Élodie", std::slice::from_ref(&range)),
            Err(Error::InvalidText)
        );
    }
    assert_eq!(candidates("one\0Private", &[]), Err(Error::InvalidText));
}
