use super::*;
pub(super) const SCOPE: &str = "https://www.googleapis.com/auth/calendar.events.readonly";
#[derive(PartialEq, Eq)]
pub(super) struct Token(pub(super) String);
impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccessToken")
    }
}
pub(super) fn decode(bytes: &[u8]) -> Result<Token, Failure> {
    #[derive(serde::Deserialize)]
    struct Reply {
        access_token: String,
        token_type: String,
        expires_in: u64,
        scope: Option<String>,
    }
    let value = vpt_protocol::read_bounded(bytes, &vpt_protocol::Limits::incoming())
        .map_err(|_| Failure::Invalid)?;
    let reply: Reply = serde_json::from_value(value).map_err(|_| Failure::Invalid)?;
    let scope = reply.scope.ok_or(Failure::Scope)?;
    if scope.split_ascii_whitespace().count() == 0
        || scope.split_ascii_whitespace().any(|s| s != SCOPE)
    {
        return Err(Failure::Scope);
    }
    if !reply.token_type.eq_ignore_ascii_case("bearer")
        || reply.expires_in == 0
        || reply.access_token.is_empty()
        || !reply
            .access_token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~+/=".contains(&b))
    {
        return Err(Failure::Invalid);
    }
    Ok(Token(reply.access_token))
}
#[cfg(test)]
mod tests;
