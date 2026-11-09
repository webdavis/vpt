mod spans;
use super::{direct::spend, *};

pub(super) fn assemble(request: &Request<'_>) -> Result<EvidencePack, BriefError> {
    let selection = select(request)?;
    let mut pack = EvidencePack {
        selection,
        unresolved: Unresolved {
            reviewed: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut bytes = 32 * 1024 * 1024;
    let mut operations = 8_388_608;
    let matches: std::collections::BTreeMap<_, _> = pack
        .selection
        .decisions
        .iter()
        .filter_map(|decision| {
            decision
                .matched
                .as_ref()
                .map(|matched| (decision.recording.as_str(), matched))
        })
        .collect();
    for index in &pack.selection.selected {
        let row = &request.candidates[*index];
        let source = row.source.as_ref().ok_or(BriefError::InvalidInput)?;
        let matched = matches
            .get(row.recording.as_str())
            .copied()
            .ok_or(BriefError::InvalidInput)?;
        spend(&mut bytes, source.note.as_os_str().len() + 62)?;
        pack.sources.push(SourceDigest {
            recording: row.recording.clone(),
            note: source.note.clone(),
            digest: source.digest,
        });
        let count = flags::unresolved(source);
        pack.unresolved.flags += count;
        pack.unresolved.notes_with_flags += usize::from(count != 0);
        pack.unresolved.reviewed &= flags::reviewed(source);
        let spans = spans::collect(source, matched, &mut operations)?;
        pack.omitted_spans += spans
            .len()
            .saturating_sub(request.options.max_spans_per_note);
        let total_words = source
            .transcript
            .segments
            .iter()
            .map(|s| s.words.len())
            .sum();
        for span in spans.into_iter().take(request.options.max_spans_per_note) {
            let first = span.occurrences.first().ok_or(BriefError::InvalidInput)?.0;
            let segment = &source.transcript.segments[first];
            let mut uncertain = false;
            for (index, words) in span.occurrences {
                uncertain |= flags::uncertain(source, index, words, total_words, &mut operations)?;
            }
            if pack.items.len() == 65_536 {
                return Err(BriefError::CapacityExceeded);
            }
            spend(
                &mut bytes,
                segment.text.len() + source.note.as_os_str().len() + 64,
            )?;
            pack.items.push(Quote {
                text: segment.text.clone(),
                certainty: if uncertain {
                    Certainty::Unverified
                } else {
                    Certainty::Confirmed
                },
                source: QuoteSource {
                    recording: row.recording.clone(),
                    at_secs: segment.start,
                    note: source.note.clone(),
                    digest: source.digest,
                },
                selector: matched.selector,
            });
        }
    }
    Ok(pack)
}
