#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    pub sent: u32,
    pub failed: u32,
    pub refused: u32,
}

impl Counts {
    pub(crate) fn json(self) -> serde_json::Value {
        serde_json::json!({"sent": self.sent, "failed": self.failed, "refused": self.refused})
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
        }
    }
}

impl std::ops::AddAssign for Counts {
    fn add_assign(&mut self, other: Self) {
        self.sent += other.sent;
        self.failed += other.failed;
        self.refused += other.refused;
    }
}
