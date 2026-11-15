use super::*;

#[test]
fn the_sentence_read_back_word_for_word_is_heard() {
    let heard = heard(SENTENCE);
    assert_eq!(heard.found, heard.expected);
    assert!(heard.enough());
}

#[test]
fn case_and_punctuation_are_ignored() {
    let shouted = SENTENCE.to_uppercase().replace('.', "!").replace(' ', " ,");
    assert_eq!(heard(&shouted).found, heard(SENTENCE).expected);
}

#[test]
fn two_thirds_of_the_words_are_enough_and_fewer_are_not() {
    let words: Vec<&str> = SENTENCE.split_whitespace().collect();
    let expected = heard(SENTENCE).expected;
    let enough = (expected * 2).div_ceil(3);
    let partial = |count: usize| heard(&words[..count].join(" "));
    let at = (1..=words.len())
        .find(|count| partial(*count).found >= enough)
        .unwrap();
    assert!(partial(at).enough());
    assert!(!partial(at - 1).enough());
}

#[test]
fn unrelated_words_are_not_heard() {
    let heard = heard("an entirely different remark about weather");
    assert!(!heard.enough());
}

#[test]
fn an_empty_transcript_hears_nothing() {
    assert_eq!(heard("").found, 0);
}
