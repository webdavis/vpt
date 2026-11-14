use vpt_application::Settings;

/// `withdrawn` is `None` when no withdraw command is set, so the output then has no such key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    pub sent: u32,
    pub failed: u32,
    pub refused: u32,
    pub withdrawn: Option<u32>,
}

impl Counts {
    pub(crate) fn start(settings: &Settings) -> Self {
        Self {
            withdrawn: (!settings.actions_withdraw_command.is_empty()).then_some(0),
            ..Self::default()
        }
    }

    pub(crate) fn is_empty(self) -> bool {
        self.sent + self.failed + self.refused + self.withdrawn.unwrap_or_default() == 0
    }

    pub(crate) fn json(self) -> serde_json::Value {
        let mut json =
            serde_json::json!({"sent": self.sent, "failed": self.failed, "refused": self.refused});
        if let Some(withdrawn) = self.withdrawn {
            json["withdrawn"] = withdrawn.into();
        }
        json
    }

    pub(crate) fn parse(value: &serde_json::Value) -> Self {
        let field = |name: &str| {
            value[name]
                .as_u64()
                .and_then(|count| u32::try_from(count).ok())
                .unwrap_or_default()
        };
        Self {
            sent: field("sent"),
            failed: field("failed"),
            refused: field("refused"),
            withdrawn: value.get("withdrawn").map(|_| field("withdrawn")),
        }
    }
}

impl std::ops::AddAssign for Counts {
    fn add_assign(&mut self, other: Self) {
        self.sent += other.sent;
        self.failed += other.failed;
        self.refused += other.refused;
        self.withdrawn = match (self.withdrawn, other.withdrawn) {
            (None, None) => None,
            (mine, theirs) => Some(mine.unwrap_or_default() + theirs.unwrap_or_default()),
        };
    }
}
