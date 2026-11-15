#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerIdentity {
    pub database_id: String,
    pub write_counter: i64,
    /// The tokens of the most recent writes, oldest first.
    pub recent_tokens: Vec<String>,
}

/// The last write this machine made to a ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteMark {
    pub database_id: String,
    pub write_counter: i64,
    pub write_token: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finding {
    Replaced,
    Behind,
    Diverged,
}

impl Finding {
    /// A higher counter whose tokens hold this machine's last write is another machine's later
    /// write, so it is no finding.
    pub fn of(ledger: Option<&LedgerIdentity>, mark: Option<&WriteMark>) -> Option<Self> {
        let mark = mark?;
        let Some(ledger) = ledger else {
            return Some(Self::Replaced);
        };
        if ledger.database_id != mark.database_id {
            Some(Self::Replaced)
        } else if ledger.write_counter < mark.write_counter {
            Some(Self::Behind)
        } else if !ledger.recent_tokens.contains(&mark.write_token) {
            Some(Self::Diverged)
        } else {
            None
        }
    }
}

impl LedgerIdentity {
    pub fn latest(&self) -> Option<WriteMark> {
        self.recent_tokens.last().map(|token| WriteMark {
            database_id: self.database_id.clone(),
            write_counter: self.write_counter,
            write_token: token.clone(),
        })
    }
}
