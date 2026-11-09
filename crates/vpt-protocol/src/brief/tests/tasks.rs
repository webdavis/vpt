use super::*;

#[test]
fn brief_output_is_bounded_after_json_escape_expansion() {
    let occasion = occasion();
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let pack = assemble(&Request {
        occasion: &occasion,
        terms: &[],
        candidates: &[],
        now: UtcInstant { secs: 1000 },
        options: Options::default(),
    })
    .unwrap();
    let context = ContextSummary {
        calendar: ContextStatus::Disabled,
        tasks: ContextStatus::Present,
    };
    for escaped in [false, true] {
        let content = if escaped {
            format!("x{}", "\u{1}".repeat(65_535))
        } else {
            "x".repeat(65_536)
        };
        let tasks: Vec<_> = (0..90)
            .map(|index| ContextTask {
                source: OccasionSource::Dam,
                id: format!("task-{index}"),
                project: "project".into(),
                content: content.clone(),
                due: None,
                priority: None,
                completed: false,
            })
            .collect();
        let document = Document {
            id: &id,
            occasion: &occasion,
            source: OccasionSource::Manual,
            pack: &pack,
            context: &context,
            tasks: &tasks,
            generated_at: UtcInstant { secs: 1000 },
        };
        document.validate().unwrap();
        if escaped {
            assert_eq!(encode(&document), Err(EncodeError::CapacityExceeded));
        } else {
            assert!(encode(&document).unwrap().len() > 65_536);
        }
    }
}

#[test]
fn json_keeps_open_task_content_and_source_separate_from_transcript_quotes() {
    let occasion = occasion();
    let id = OccasionId::derive(&occasion.at, &Sha256Digest([7; 32]));
    let pack = assemble(&Request {
        occasion: &occasion,
        terms: &[],
        candidates: &[],
        now: UtcInstant { secs: 1000 },
        options: Options::default(),
    })
    .unwrap();
    let context = ContextSummary {
        calendar: ContextStatus::Disabled,
        tasks: ContextStatus::Present,
    };
    let task = ContextTask {
        source: OccasionSource::Dam,
        id: "private-task".into(),
        project: "private-project".into(),
        content: "Exact task\ncontent".into(),
        due: Some("2026-10-02".into()),
        priority: Some(4),
        completed: false,
    };
    let tasks = [
        task.clone(),
        ContextTask {
            id: "done-task".into(),
            completed: true,
            ..task
        },
    ];
    let document = Document {
        id: &id,
        occasion: &occasion,
        source: OccasionSource::Manual,
        pack: &pack,
        context: &context,
        tasks: &tasks,
        generated_at: UtcInstant { secs: 1000 },
    };
    let value: serde_json::Value = serde_json::from_str(&encode(&document).unwrap()).unwrap();
    assert_eq!(
        value["tasks"],
        serde_json::json!([{"source":"dam","id":"private-task","project":"private-project","content":"Exact task\ncontent","due":"2026-10-02","priority":4,"completed":false}])
    );
    assert!(value["items"].as_array().unwrap().is_empty());
}
