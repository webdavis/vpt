use super::*;
use vpt_domain::time::UtcInstant;

#[test]
fn native_client_enforces_https_rustls_and_no_redirect_or_proxy() {
    let client = Native::new();
    let config = client.agent.config();
    assert!(config.https_only());
    assert_eq!(config.max_redirects(), 0);
    assert!(config.proxy().is_none());
    assert!(!config.tls_config().disable_verification());
    assert_eq!(
        config.tls_config().provider(),
        ureq::tls::TlsProvider::Rustls
    );
    assert!(matches!(
        client.agent.get("http://invalid.test/").call(),
        Err(ureq::Error::RequireHttpsOnly(_))
    ));
}

#[test]
fn oauth_form_and_calendar_paths_encode_values_without_changing_the_endpoint() {
    let credentials = GoogleCredentials {
        client_id: "id & secret".into(),
        client_secret: "a+b".into(),
        refresh_token: "PRIVATE/refresh=".into(),
    };
    let refresh = request::build(Request::Refresh(&credentials)).unwrap();
    assert_eq!(refresh.method(), ureq::http::Method::POST);
    assert_eq!(
        refresh.uri().to_string(),
        "https://oauth2.googleapis.com/token"
    );
    let body = String::from_utf8(refresh.body().clone()).unwrap();
    assert!(body.contains("client_id=id%20%26%20secret"));
    assert!(body.contains("client_secret=a%2Bb"));
    assert!(body.contains("refresh_token=PRIVATE%2Frefresh%3D"));
    assert!(body.contains("grant_type=refresh_token"));
    assert!(
        body.contains("scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.events.readonly")
    );
    let events = request::build(Request::Events {
        token: "PRIVATE-token",
        calendar: "cal/../?x=1@example.org",
        window: ContextWindow {
            start: UtcInstant { secs: 1000 },
            end: UtcInstant { secs: 2000 },
        },
        page: Some("next&private=1"),
    })
    .unwrap();
    assert_eq!(events.method(), ureq::http::Method::GET);
    assert_eq!(events.uri().host(), Some("www.googleapis.com"));
    assert!(
        events
            .uri()
            .path()
            .contains("cal%2F..%2F%3Fx%3D1%40example.org/events")
    );
    let query = events.uri().query().unwrap();
    for part in [
        "singleEvents=true",
        "timeMin=1970-01-01T00%3A16%3A40Z",
        "timeMax=1970-01-01T00%3A33%3A20Z",
        "pageToken=next%26private%3D1",
    ] {
        assert!(query.contains(part), "{query}");
    }
    assert!(events.headers()["authorization"].is_sensitive());
    assert!(events.body().is_empty());
}

#[test]
fn response_body_limits_and_non_success_statuses_never_expose_provider_text() {
    fn response(status: u16, bytes: Vec<u8>) -> ureq::http::Response<ureq::Body> {
        ureq::http::Response::builder()
            .status(status)
            .body(ureq::Body::builder().data(bytes))
            .unwrap()
    }
    assert_eq!(read(response(200, b"{}".to_vec())).unwrap(), b"{}");
    assert_eq!(
        read(response(200, vec![b'x'; 65_536])).unwrap().len(),
        65_536
    );
    assert_eq!(
        read(response(200, vec![b'x'; 65_537])),
        Err(Failure::Capacity)
    );
    for status in [301, 302, 307, 401, 403, 429, 500] {
        assert_eq!(
            read(response(status, b"PRIVATE token response".to_vec())),
            Err(Failure::HttpStatus)
        );
    }
}
