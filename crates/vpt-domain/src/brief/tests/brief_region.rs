use crate::brief::markdown::{END, Region, START};
use crate::note::RenderError;

#[test]
fn brief_region_rewrite_preserves_operator_bytes_and_its_original_line_endings() {
    let source = format!(
        "# Private heading\r\nOperator above\r\n{START}\r\nOld body\r\n{END}\r\nOperator below\r\n"
    );
    let region = Region::parse(&source).unwrap();
    assert_eq!(region.content(), "Old body\r\n");
    let updated = region.rewrite("New body").unwrap();
    assert_eq!(
        updated,
        format!(
            "# Private heading\r\nOperator above\r\n{START}\r\nNew body\n{END}\r\nOperator below\r\n"
        )
    );
    assert_eq!(Region::parse(&updated).unwrap().content(), "New body\n");
}

#[test]
fn missing_duplicate_reversed_and_inline_brief_markers_are_refused() {
    for source in [
        format!("{START}\nBody"),
        format!("{END}\n{START}"),
        format!("{START}\n{START}\n{END}"),
        format!("prefix {START}\n{END}"),
        format!("{START} trailing\n{END}"),
        format!("{START}\n{END}\n{END}"),
    ] {
        assert!(matches!(Region::parse(&source), Err(RenderError::Markers)));
    }
    let source = format!("{START}\nBody\n{END}");
    let region = Region::parse(&source).unwrap();
    assert!(matches!(
        region.rewrite(&format!("Injected {END}")),
        Err(RenderError::Markers)
    ));
}
