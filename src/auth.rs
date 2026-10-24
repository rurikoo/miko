use anyhow::{Context, Result, bail};
use reqwest::{
    blocking::Client,
    header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DispenserResponse {
    email: Option<String>,
    #[serde(
        alias = "auth",
        alias = "token",
        alias = "authToken",
        alias = "auth_token"
    )]
    auth_token: Option<String>,
}

/// Goes to the public Aurora Store dispenser for email and token
pub fn fetch_public_dispenser_auth() -> Result<(String, String)> {
    let dispenser_url = "https://auroraoss.com/api/auth";
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("com.aurora.store-4.8.4-76"),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    let client = Client::builder().default_headers(headers).build()?;
    let response = client.get(dispenser_url).send()?;
    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        bail!("received 403, request got blocked");
    } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
    {
        bail!("received 429 or 503 :(");
    } else if !status.is_success() {
        bail!("received unexpected HTTP status: {status}");
    }

    let body = response
        .text()
        .context("failed to read dispenser response")?;
    parse_dispenser_response(&body)
}

fn parse_dispenser_response(body: &str) -> Result<(String, String)> {
    let payload: DispenserResponse =
        serde_json::from_str(body).context("failed to parse JSON response from dispenser")?;

    let email = payload
        .email
        .context("dispenser did not return email field")?;
    let token = payload
        .auth_token
        .context("dispenser did not return auth token field")?;

    if !token.starts_with("ya29.") {
        bail!("invalid auth token received: {token}");
    }

    Ok((email, token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispenser_accepts_auth_token_field() {
        let body = r#"{"email":"a@b.c","auth_token":"ya29.abc"}"#;
        let (email, token) = parse_dispenser_response(body).unwrap();
        assert_eq!(email, "a@b.c");
        assert_eq!(token, "ya29.abc");
    }

    #[test]
    fn dispenser_accepts_each_alias() {
        for key in ["auth", "token", "authToken", "auth_token"] {
            let body = format!(r#"{{"email":"a@b.c","{key}":"ya29.x"}}"#);
            let (_, token) = parse_dispenser_response(&body)
                .unwrap_or_else(|e| panic!("alias {key} failed: {e}"));
            assert_eq!(token, "ya29.x");
        }
    }

    #[test]
    fn dispenser_rejects_token_without_ya29_prefix() {
        let body = r#"{"email":"a@b.c","auth":"notatoken"}"#;
        let err = parse_dispenser_response(body).unwrap_err().to_string();
        assert!(err.contains("invalid auth token"));
    }

    #[test]
    fn dispenser_missing_email_is_an_error() {
        let body = r#"{"auth":"ya29.x"}"#;
        let err = parse_dispenser_response(body).unwrap_err().to_string();
        assert!(err.contains("did not return email"));
    }

    #[test]
    fn dispenser_missing_token_is_an_error() {
        let body = r#"{"email":"a@b.c"}"#;
        let err = parse_dispenser_response(body).unwrap_err().to_string();
        assert!(err.contains("did not return auth token"));
    }

    #[test]
    fn dispenser_bad_json_is_an_error() {
        let err = parse_dispenser_response("<html>nope</html>")
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed to parse JSON"));
    }

    /// ignored by default
    #[test]
    #[ignore]
    fn live_dispenser_returns_token() {
        let (email, token) = fetch_public_dispenser_auth().unwrap();
        assert!(!email.is_empty());
        assert!(token.starts_with("ya29."));
    }
}
