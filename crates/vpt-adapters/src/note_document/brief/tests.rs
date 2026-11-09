use super::*;
use std::path::Path;
use vpt_domain::{
    brief::{self, ContextSummary, Options, Request, document, markdown},
    digest::Sha256Digest,
    note::{document::Profile, links::LinkStyle},
    occasion::{OccasionDetails, OccasionSource, time::OccasionTime},
    time::UtcInstant,
};

fn document(title: &str, tags: &[&str]) -> (OccasionId, String) {
    let occasion = OccasionDetails {
        at: OccasionTime::parse("2026-10-01T09:00:00Z").unwrap(),
        title: title.into(),
        duration_secs: None,
        participants: vec![],
        tags: tags.iter().map(|s| (*s).into()).collect(),
    };
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let now = UtcInstant { secs: 1000 };
    let pack = brief::assemble(&Request {
        occasion: &occasion,
        terms: &[],
        candidates: &[],
        now,
        options: Options::default(),
    })
    .unwrap();
    let context = ContextSummary::default();
    let text = markdown::create(&markdown::Note {
        document: document::Document {
            id: &id,
            source: OccasionSource::Manual,
            occasion: &occasion,
            pack: &pack,
            context: &context,
            tasks: &[],
            generated_at: now,
        },
        filename: "2026-10-01-planning-07070707.md",
        origin: Path::new("/vault/briefs"),
        link_style: LinkStyle::Markdown,
        wiki_root: None,
        profile: Profile::Portable,
    })
    .unwrap();
    (id, text)
}

#[test]
fn rewriting_a_brief_preserves_operator_text_properties_and_tags() {
    let (id, source) = document("Private old title", &["planning"]);
    let source = source.replacen("---\n","---\nstatus: archived\nstartDate: '2025-01-02'\ndescription: |\n  Private description\ncustom: 'Private property' # keep\nvptOld: stale\n",1)
        .replace("  - \"planning\"\n","  - \"planning\"\n  - \"personal\"\n")
        .replace(markdown::START,&format!("Private prose above\n{}",markdown::START));
    let source = format!("{source}Private prose below\n");
    let (_, replacement) = document("Private new title", &["next"]);
    let parsed = Document::parse(&source).unwrap();
    parsed.check_owner(&id).unwrap();
    let updated = parsed.rewrite(&replacement, &["planning".into()]).unwrap();
    for preserved in [
        "status: archived\n",
        "startDate: '2025-01-02'\n",
        "description: |\n  Private description\n",
        "custom: 'Private property' # keep\n",
        "Private prose above\n",
        "Private prose below\n",
    ] {
        assert!(updated.contains(preserved), "{preserved}");
    }
    assert!(!updated.contains("vptOld"));
    assert!(!updated.contains("Private old title"));
    assert!(updated.contains("Private new title"));
    assert!(updated.contains("personal"));
    assert!(updated.contains("next"));
    assert!(!updated.contains("\"planning\""));
    Document::parse(&updated).unwrap().check_owner(&id).unwrap();
    assert_eq!(
        Document::parse(&updated).unwrap().content().unwrap(),
        Document::parse(&replacement).unwrap().content().unwrap()
    );
}

#[test]
fn brief_rewrites_refuse_foreign_identity_schema_and_broken_markers() {
    let (id, source) = document("Private title", &[]);
    let foreign = OccasionId::parse("2026-10-01T090000-080808080808").unwrap();
    let parsed = Document::parse(&source).unwrap();
    assert_eq!(parsed.check_owner(&foreign), Err(NoteError::Identity));
    assert_eq!(
        parsed.rewrite(&source.replace(id.as_str(), foreign.as_str()), &[]),
        Err(NoteError::Identity)
    );
    let newer = source.replace("vptSchema: 1", "vptSchema: 2");
    let newer = Document::parse(&newer).unwrap();
    assert_eq!(newer.check_owner(&id), Err(NoteError::UnsupportedSchema(2)));
    assert_eq!(
        newer.rewrite(&source, &[]),
        Err(NoteError::UnsupportedSchema(2))
    );
    for source in [
        source.replace(markdown::END, ""),
        source.replace(
            markdown::START,
            &format!("{}\n{}", markdown::START, markdown::START),
        ),
        source.replace("vptKind: \"brief\"", "vptKind: \"analysis\""),
    ] {
        assert!(Document::parse(&source).is_err());
    }
}

#[test]
fn flow_frontmatter_and_duplicate_identity_keys_follow_the_same_ownership_rules() {
    let (id, source) = document("Private old title", &[]);
    let body = source.splitn(3, "---\n").nth(2).unwrap();
    let source = format!(
        "---\n{{vptSchema: 1, vptKind: brief, vptOccasion: '{}', tags: [personal], custom: 'Private property'}}\n---\n{body}",
        id.as_str()
    );
    let (_, replacement) = document("Private new title", &["planning"]);
    let updated = Document::parse(&source)
        .unwrap()
        .rewrite(&replacement, &[])
        .unwrap();
    assert!(updated.contains("custom: 'Private property'"));
    assert!(updated.contains("personal"));
    assert!(updated.contains("Private new title"));
    Document::parse(&updated).unwrap().check_owner(&id).unwrap();
    let duplicate = source.replace(
        "vptOccasion:",
        &format!("vptOccasion: '{}', vptOccasion:", id.as_str()),
    );
    assert!(matches!(
        Document::parse(&duplicate),
        Err(NoteError::Frontmatter)
    ));
}
