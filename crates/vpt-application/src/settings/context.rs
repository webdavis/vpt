#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextSource {
    None,
    Dam,
    Google,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ContextSettings {
    pub source: ContextSource,
    pub calendars: Vec<String>,
    pub projects: Vec<String>,
    pub dam_command: Vec<String>,
    pub dam_max_age: Vec<String>,
    pub google: GoogleCredentials,
}
#[derive(Clone, PartialEq, Eq)]
pub struct GoogleCredentials {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
}
impl std::fmt::Debug for ContextSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextSettings")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for GoogleCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GoogleCredentials")
    }
}
