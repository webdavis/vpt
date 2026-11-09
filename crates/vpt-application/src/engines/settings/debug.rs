use super::EngineKind;
use std::fmt;

impl fmt::Debug for EngineKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Apple => formatter.write_str("Apple"),
            Self::Whisply {
                command,
                device,
                model,
            } => formatter
                .debug_struct("Whisply")
                .field("argument_count", &command.len())
                .field("device", device)
                .field("model", model)
                .finish(),
            Self::Command {
                command,
                family,
                local,
            } => formatter
                .debug_struct("Command")
                .field("argument_count", &command.len())
                .field("family", family)
                .field("local", local)
                .finish(),
        }
    }
}
