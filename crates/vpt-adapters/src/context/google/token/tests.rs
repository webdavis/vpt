use super::*;
use serde_json::json;

#[test]
fn token_requires_the_exact_read_only_scope_and_hides_credentials_in_debugging() {
    let bytes = json!({"access_token":"PRIVATE-token","token_type":"Bearer","expires_in":3600,"scope":SCOPE}).to_string();
    let token = decode(bytes.as_bytes()).unwrap();
    assert_eq!(token.0, "PRIVATE-token");
    assert!(!format!("{token:?}").contains("PRIVATE"));
    for scope in [
        "",
        "https://www.googleapis.com/auth/calendar",
        "https://www.googleapis.com/auth/calendar.readonly",
    ] {
        let bytes = json!({"access_token":"PRIVATE-token","token_type":"Bearer","expires_in":3600,"scope":scope}).to_string();
        assert_eq!(decode(bytes.as_bytes()), Err(Failure::Scope));
    }
    let bytes = bytes.replace(
        SCOPE,
        &format!("{SCOPE} https://www.googleapis.com/auth/calendar.events"),
    );
    assert_eq!(decode(bytes.as_bytes()), Err(Failure::Scope));
}

#[test]
fn missing_scope_expired_token_control_characters_and_oversized_replies_are_refused_privately() {
    for value in [
        json!({"access_token":"PRIVATE-token","token_type":"Bearer","expires_in":3600}),
        json!({"access_token":"PRIVATE-token","token_type":"Basic","expires_in":3600,"scope":SCOPE}),
        json!({"access_token":"PRIVATE-token","token_type":"Bearer","expires_in":0,"scope":SCOPE}),
        json!({"access_token":"PRIVATE\r\nheader","token_type":"Bearer","expires_in":3600,"scope":SCOPE}),
        json!({"error":"PRIVATE provider response"}),
    ] {
        let error = decode(value.to_string().as_bytes()).unwrap_err();
        assert!(!format!("{error:?}").contains("PRIVATE"));
    }
    assert!(decode(&vec![b' '; 65_537]).is_err());
}
