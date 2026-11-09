use std::borrow::Cow;
use vpt_domain::normalize::canonical;

pub(super) fn with_learned<'a>(terms: &'a [String], learned: Option<&str>) -> Cow<'a, [String]> {
    let Some(learned) = learned else {
        return Cow::Borrowed(terms);
    };
    let key = canonical(learned);
    if terms.iter().any(|term| {
        canonical(term)
            .split_whitespace()
            .eq(key.split_whitespace())
    }) {
        return Cow::Borrowed(terms);
    }
    let mut extended = terms.to_vec();
    extended.push(learned.into());
    Cow::Owned(extended)
}

#[cfg(test)]
mod tests;
