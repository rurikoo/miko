use anyhow::anyhow;
use gpapi::error::ErrorKind as GpapiErrorKind;

/// Turns gpapi errors into messages that say what to do next
pub(crate) fn map_gpapi_error(e: gpapi::error::Error) -> anyhow::Error {
    match e.kind() {
        GpapiErrorKind::Authentication => {
            anyhow!("google rejected the dispenser token, try again for a new one")
        }
        GpapiErrorKind::TermsOfService => {
            anyhow!("this account must accept google play's terms of service first")
        }
        _ => anyhow!("gpapi error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpapi::error::{Error as GpapiError, ErrorKind as Kind};

    #[test]
    fn auth_error_maps_to_friendly_message() {
        let e = GpapiError::new(Kind::Authentication);
        let msg = map_gpapi_error(e).to_string();
        assert!(msg.contains("dispenser token"));
    }

    #[test]
    fn tos_error_maps_to_friendly_message() {
        let e = GpapiError::new(Kind::TermsOfService);
        let msg = map_gpapi_error(e).to_string();
        assert!(msg.contains("terms of service"));
    }

    #[test]
    fn other_errors_keep_their_text() {
        let e = GpapiError::new(Kind::Str("boom".to_string()));
        let msg = map_gpapi_error(e).to_string();
        assert!(msg.contains("boom"));
    }
}
