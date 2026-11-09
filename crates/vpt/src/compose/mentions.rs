use vpt_adapters::{Roots, known_terms, note_index::Index};
use vpt_application::{
    Settings,
    ports::{MentionDerivation, NoteInputs, NoteRelation},
};
use vpt_domain::relations::{Kind, MentionAmbiguity, Provenance, Relation, Rule, State, Target};
use vpt_protocol::error::{ErrorDocument, ErrorKind};

mod terms;

pub(crate) struct Context {
    index: Index,
    terms: Vec<String>,
}
impl Context {
    pub(crate) fn index(&self) -> &Index {
        &self.index
    }
    pub(crate) fn read(settings: &Settings, roots: &Roots) -> Result<Self, Box<ErrorDocument>> {
        Self::with_index(super::index::read(settings, roots)?, settings)
    }
    pub(crate) fn with_index(
        index: Index,
        settings: &Settings,
    ) -> Result<Self, Box<ErrorDocument>> {
        let terms = known_terms::read(&settings.reconcile.known_terms_path)
            .map_err(super::publication::vocabulary::term_error)?;
        Ok(Self { index, terms })
    }
    pub(crate) fn enrich(
        &self,
        input: &mut NoteInputs,
        learned: Option<&str>,
    ) -> Result<(), Box<ErrorDocument>> {
        let Some(accepted) = &input.review.accepted else {
            return Ok(());
        };
        let terms = terms::with_learned(&self.terms, learned);
        let words: Vec<_> = accepted
            .transcript
            .segments
            .iter()
            .flat_map(|segment| &segment.words)
            .take(65_537)
            .map(|word| word.text.as_str())
            .collect();
        let matches = self
            .index
            .mentions(&words, &terms)
            .map_err(|error| Box::new(super::errors::store_error(error)))?;
        let mut links = Vec::new();
        for mention in matches.links {
            let path = &self
                .index
                .notes()
                .get(mention.target)
                .ok_or_else(inconsistent)?
                .path;
            let target = path.to_str().ok_or_else(inconsistent)?.to_owned();
            links.push(NoteRelation {
                path: Some(path.clone()),
                relation: Relation {
                    kind: Kind::Mentions,
                    target: Target::Note(target),
                    state: State::Confirmed,
                    provenance: Provenance::Derived(Rule::KnownTerm),
                    ranges: mention.ranges,
                },
            });
        }
        let mut ambiguities = Vec::new();
        for ambiguity in matches.ambiguous {
            ambiguities.push(MentionAmbiguity {
                term: terms.get(ambiguity.term).ok_or_else(inconsistent)?.clone(),
                ranges: ambiguity.ranges,
            });
        }
        input.derived_mentions = Some(MentionDerivation { links, ambiguities });
        Ok(())
    }
}
fn inconsistent() -> Box<ErrorDocument> {
    Box::new(ErrorDocument::new(
        ErrorKind::Ledger,
        "mention resolution is inconsistent",
    ))
}
