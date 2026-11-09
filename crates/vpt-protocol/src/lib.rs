mod additive;
pub mod context;
pub mod engine;
pub mod error;
mod event;
mod helper;
mod limits;
pub mod result;
pub mod whisply;

pub use event::{EventEncodeError, encode_history, encode_hook};
pub use helper::{HELPER_SCHEMA, HelperVersion, Trashed};
pub use limits::{LimitViolation, Limits, read_bounded};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_reader_and_reply_types_compose() {
        let value = read_bounded(
            br#"{"schema":"vpt.helper/1","version":"1.2.3-rc.1"}"#,
            &Limits::incoming(),
        )
        .expect("bounded version document");
        let reply: HelperVersion = serde_json::from_value(value).expect("version reply");
        assert_eq!(reply.schema, HELPER_SCHEMA);
        assert_eq!(reply.major(), Some(1));

        let value = read_bounded(br#"{"trashed":"/sandbox/file"}"#, &Limits::incoming())
            .expect("bounded trash document");
        let reply: Trashed = serde_json::from_value(value).expect("trash reply");
        assert_eq!(reply.trashed, "/sandbox/file");
    }
}
