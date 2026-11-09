use super::{MAX_BYTES, MAX_ITEMS, invalid};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};
use vpt_application::ports::LedgerError;
use vpt_domain::relations::{Kind, Provenance, Relation, Rule, State, Target};

pub(super) fn relations(relations: &[Relation]) -> Result<(), LedgerError> {
    if relations.len() > MAX_ITEMS {
        return Err(invalid());
    }
    let mut keys = BTreeSet::new();
    let mut bytes = MAX_BYTES;
    let mut ranges = MAX_ITEMS;
    for relation in relations {
        let target = relation.target.as_str();
        text(target, &mut bytes)?;
        if !keys.insert((relation.kind, target)) {
            return Err(invalid());
        }
        if let Target::Note(path) = &relation.target {
            let path = Path::new(path);
            if !path.is_absolute()
                || path.file_name().is_none()
                || path.components().collect::<PathBuf>().as_os_str() != path.as_os_str()
                || path
                    .components()
                    .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            {
                return Err(invalid());
            }
        }
        if matches!(relation.kind, Kind::Continues | Kind::Supersedes)
            && !matches!(relation.target, Target::Recording(_))
        {
            return Err(invalid());
        }
        match &relation.provenance {
            Provenance::Manual => {}
            Provenance::Derived(rule) => {
                let valid = match rule {
                    Rule::ImmediatePredecessor => relation.kind == Kind::Continues,
                    Rule::KnownTerm => {
                        relation.kind == Kind::Mentions
                            && !relation.ranges.is_empty()
                            && matches!(relation.target, Target::Note(_))
                    }
                };
                if !valid || relation.state != State::Confirmed {
                    return Err(invalid());
                }
            }
            Provenance::Proposal { by, reason, .. } => {
                if relation.kind != Kind::Related || by.trim().is_empty() {
                    return Err(invalid());
                }
                text(by, &mut bytes)?;
                text(reason, &mut bytes)?;
            }
        }
        if relation.kind == Kind::Supersedes && relation.provenance != Provenance::Manual {
            return Err(invalid());
        }
        if relation.kind != Kind::Mentions && !relation.ranges.is_empty() {
            return Err(invalid());
        }
        super::ranges::validate(&relation.ranges, &mut ranges)?;
    }
    Ok(())
}

fn text(value: &str, bytes: &mut usize) -> Result<(), LedgerError> {
    if value.len() > 4 * MAX_ITEMS
        || value.chars().count() > MAX_ITEMS
        || value
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\t' | '\n' | '\r'))
    {
        return Err(invalid());
    }
    *bytes = bytes.checked_sub(value.len()).ok_or_else(invalid)?;
    Ok(())
}
