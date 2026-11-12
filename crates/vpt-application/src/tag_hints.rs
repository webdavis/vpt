use crate::ports::TagRename;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap, HashSet},
};
use vpt_domain::tags::{State, key};

pub const KNOWN_TAGS: &str = "{known_tags}";
pub const REFUSED_TAGS: &str = "{refused_tags}";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TagHints {
    pub known: Vec<KnownTag>,
    pub refused: Vec<RefusedTag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownTag {
    pub name: String,
    pub notes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedTag {
    pub name: String,
    pub renamed_to: Option<String>,
}

pub fn wanted(command: &[String]) -> bool {
    command
        .iter()
        .any(|argument| argument.contains(KNOWN_TAGS) || argument.contains(REFUSED_TAGS))
}

pub fn hints<'a>(
    notes: impl IntoIterator<Item = &'a [String]>,
    vocabulary: &[String],
    saved: &[(String, State)],
    renames: &[TagRename],
) -> TagHints {
    let mut known: HashMap<String, KnownTag> = HashMap::new();
    for tags in notes {
        let mut seen = HashSet::new();
        for name in tags {
            let Ok(key) = key(name) else { continue };
            if seen.insert(key.clone()) {
                entry(&mut known, key, name).notes += 1;
            }
        }
    }
    for name in vocabulary {
        if let Ok(key) = key(name) {
            entry(&mut known, key, name);
        }
    }
    let keyed = |state| {
        saved
            .iter()
            .filter(move |(_, saved)| *saved == state)
            .filter_map(|(name, _)| Some((key(name).ok()?, name)))
    };
    let confirmed: HashSet<_> = keyed(State::Confirmed).map(|(key, _)| key).collect();
    let renames = crate::tag_renames::active(renames, saved);
    let mut refused_keys: HashSet<_> = renames.iter().filter_map(|r| key(&r.old).ok()).collect();
    let rejected: BTreeMap<_, _> = keyed(State::Rejected)
        .filter(|(key, _)| {
            !confirmed.contains(key)
                && !refused_keys.contains(key)
                && known.get(key).is_none_or(|tag| tag.notes == 0)
        })
        .collect();
    refused_keys.extend(rejected.keys().cloned());
    let mut known: Vec<_> = known
        .into_iter()
        .filter(|(key, _)| !refused_keys.contains(key))
        .collect();
    known.sort_by(|(a, x), (b, y)| (Reverse(x.notes), a).cmp(&(Reverse(y.notes), b)));
    TagHints {
        known: known.into_iter().map(|(_, tag)| tag).collect(),
        refused: renames
            .into_iter()
            .map(|rename| RefusedTag {
                name: rename.old.clone(),
                renamed_to: Some(rename.new.clone()),
            })
            .chain(rejected.into_values().map(|name| RefusedTag {
                name: name.clone(),
                renamed_to: None,
            }))
            .collect(),
    }
}

fn entry<'a>(
    known: &'a mut HashMap<String, KnownTag>,
    key: String,
    name: &str,
) -> &'a mut KnownTag {
    known.entry(key).or_insert_with(|| KnownTag {
        name: name.to_owned(),
        notes: 0,
    })
}
