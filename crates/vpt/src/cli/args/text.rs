#[derive(Clone, PartialEq, Eq)]
pub struct PrivateText(String);
impl PrivateText {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl From<String> for PrivateText {
    fn from(text: String) -> Self {
        Self(text)
    }
}
impl std::fmt::Debug for PrivateText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PrivateText")
    }
}
