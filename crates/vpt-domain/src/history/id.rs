#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LedgerNamespace([u8; 16]);

impl LedgerNamespace {
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn bytes(self) -> [u8; 16] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RunId {
    namespace: LedgerNamespace,
    sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EventId {
    namespace: LedgerNamespace,
    sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExportFailureId {
    namespace: LedgerNamespace,
    sequence: u64,
}

impl EventId {
    pub const fn new(namespace: LedgerNamespace, sequence: u64) -> Self {
        Self {
            namespace,
            sequence,
        }
    }

    pub const fn namespace(self) -> LedgerNamespace {
        self.namespace
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub fn render(self) -> String {
        render_id(self.namespace, self.sequence)
    }
}

impl RunId {
    pub const fn new(namespace: LedgerNamespace, sequence: u64) -> Self {
        Self {
            namespace,
            sequence,
        }
    }

    pub const fn namespace(self) -> LedgerNamespace {
        self.namespace
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub fn render(self) -> String {
        render_id(self.namespace, self.sequence)
    }
}

impl ExportFailureId {
    pub const fn new(namespace: LedgerNamespace, sequence: u64) -> Self {
        Self {
            namespace,
            sequence,
        }
    }

    pub const fn namespace(self) -> LedgerNamespace {
        self.namespace
    }

    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    pub fn render(self) -> String {
        render_id(self.namespace, self.sequence)
    }
}

fn render_id(namespace: LedgerNamespace, sequence: u64) -> String {
    let mut text = String::with_capacity(53);
    for byte in namespace.bytes() {
        use std::fmt::Write;
        write!(text, "{byte:02x}").expect("string write");
    }
    format!("{text}-{sequence}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_id_includes_namespace_and_sequence() {
        let namespace = LedgerNamespace::new([0xab; 16]);
        assert_eq!(
            EventId::new(namespace, 42).render(),
            "abababababababababababababababab-42"
        );
        assert_ne!(
            EventId::new(LedgerNamespace::new([0xcd; 16]), 42).render(),
            EventId::new(namespace, 42).render()
        );
    }
}
