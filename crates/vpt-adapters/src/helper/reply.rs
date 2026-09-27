use super::HelperError;
use serde_json::Value;
use vpt_application::ports::TrashError;
use vpt_protocol::HELPER_SCHEMA;

pub(super) fn schema(value: &Value) -> Result<(), HelperError> {
    let Some(schema) = value.get("schema").and_then(Value::as_str) else {
        return Err(HelperError::Unknown("invalid helper schema".into()));
    };
    if schema == HELPER_SCHEMA {
        return Ok(());
    }
    let major = schema
        .strip_prefix("vpt.helper/")
        .filter(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|part| part.parse::<u32>().ok());
    match major {
        Some(found) => Err(HelperError::Unknown(format!(
            "unsupported helper schema major {found}"
        ))),
        None => Err(HelperError::Unknown("invalid helper schema".into())),
    }
}

pub(super) fn trash_error(error: HelperError) -> TrashError {
    match error {
        HelperError::Absent => TrashError::HelperAbsent,
        HelperError::MajorMismatch { found } => TrashError::HelperVersion { found },
        HelperError::Failed(detail) => TrashError::Failed(detail),
        HelperError::Unknown(detail) => TrashError::Unknown(detail),
    }
}
