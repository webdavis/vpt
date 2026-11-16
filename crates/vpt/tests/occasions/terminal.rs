use super::styled::{piped, same_in_every_format, shown};
use super::*;

#[test]
fn occasions_at_a_terminal_mark_each_occasion_by_whether_it_is_briefed() {
    let sandbox = fixture::configured("occasions-terminal");
    assert_eq!(shown(&sandbox, &["occasions"]), "  · no occasions\n");
    let record = seed(&sandbox);
    let (id, at) = (record.id.as_str(), record.details.at.as_str());
    assert_eq!(
        shown(&sandbox, &["occasions"]),
        format!("  · {id}: {at}, unbriefed, Private\\noccasion\n")
    );
    assert_eq!(
        piped(&sandbox, &["occasions"]),
        format!("{id}  {at}  unbriefed  Private\\noccasion\n")
    );
    same_in_every_format(&sandbox, &["occasions"]);
}
