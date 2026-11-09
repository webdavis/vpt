use std::ops::Range;

#[derive(Clone, PartialEq, Eq)]
pub struct MentionAmbiguity {
    pub term: String,
    pub ranges: Vec<Range<usize>>,
}
impl std::fmt::Debug for MentionAmbiguity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MentionAmbiguity")
            .field("ranges", &self.ranges)
            .finish()
    }
}
