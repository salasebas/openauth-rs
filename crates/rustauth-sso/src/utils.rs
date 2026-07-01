#[cfg(feature = "oidc")]
use std::sync::OnceLock;

#[cfg(feature = "saml")]
use base64::Engine;
use rustauth_core::api::{json_response, ApiResponse};
use rustauth_core::auth::trusted_origins::OriginMatchSettings;
use rustauth_core::context::AuthContext;
use rustauth_core::error::RustAuthError;
use serde::Serialize;
#[cfg(feature = "saml")]
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
#[cfg(feature = "saml")]
use time::format_description::well_known::Rfc3339;
#[cfg(feature = "saml")]
use x509_parser::prelude::{FromDer, X509Certificate};
#[cfg(feature = "saml")]
use x509_parser::public_key::PublicKey;

#[cfg(feature = "oidc")]
use rustauth_oauth::oauth2::{OAuthHttpClient, OAuthHttpClientConfig};

#[cfg(feature = "saml")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateMetadata {
    pub sha256_fingerprint: String,
    pub not_before: Option<String>,
    pub not_after: Option<String>,
    pub public_key_algorithm: Option<String>,
    pub parse_error: Option<String>,
}

pub fn client_id_last_four(client_id: &str) -> String {
    let char_count = client_id.chars().count();
    if char_count <= 4 {
        return "****".to_owned();
    }
    let suffix = client_id
        .chars()
        .skip(char_count.saturating_sub(4))
        .collect::<String>();
    format!("****{suffix}")
}

#[cfg(feature = "saml")]
pub fn certificate_metadata(cert: &str) -> CertificateMetadata {
    let normalized = normalize_certificate(cert);
    let Ok(der) = base64::engine::general_purpose::STANDARD.decode(&normalized) else {
        return CertificateMetadata {
            sha256_fingerprint: String::new(),
            not_before: None,
            not_after: None,
            public_key_algorithm: None,
            parse_error: Some("Failed to parse certificate".to_owned()),
        };
    };
    let fingerprint = sha256_hex(&der);
    let Ok((_, certificate)) = X509Certificate::from_der(&der) else {
        return CertificateMetadata {
            sha256_fingerprint: fingerprint,
            not_before: None,
            not_after: None,
            public_key_algorithm: None,
            parse_error: Some("Failed to parse certificate".to_owned()),
        };
    };
    let validity = certificate.validity();
    CertificateMetadata {
        sha256_fingerprint: fingerprint,
        not_before: validity.not_before.to_datetime().format(&Rfc3339).ok(),
        not_after: validity.not_after.to_datetime().format(&Rfc3339).ok(),
        public_key_algorithm: certificate
            .public_key()
            .parsed()
            .ok()
            .map(public_key_algorithm),
        parse_error: None,
    }
}

pub fn json<T: Serialize>(
    status: http::StatusCode,
    body: &T,
) -> Result<ApiResponse, RustAuthError> {
    json_response(status, body, Vec::new())
}

/// Returns the shared OIDC HTTP client for the requested SSRF policy.
///
/// When `allow_private_ips` is `false` (the default for SSO providers) the
/// client blocks requests that resolve to private, loopback, or otherwise
/// non-public addresses. Clients are cached per policy so OIDC discovery,
/// JWKS, userinfo, and token requests share one connection pool and guard.
#[cfg(feature = "oidc")]
pub(crate) fn oauth_http_client(
    allow_private_ips: bool,
) -> Result<&'static OAuthHttpClient, RustAuthError> {
    if allow_private_ips {
        static PERMISSIVE_HTTP_CLIENT: OnceLock<Result<OAuthHttpClient, String>> = OnceLock::new();
        cached_oauth_http_client(&PERMISSIVE_HTTP_CLIENT, || {
            build_oauth_http_client(OAuthHttpClientConfig {
                allow_private_ips: true,
                ..OAuthHttpClientConfig::default()
            })
        })
    } else {
        static GUARDED_HTTP_CLIENT: OnceLock<Result<OAuthHttpClient, String>> = OnceLock::new();
        cached_oauth_http_client(&GUARDED_HTTP_CLIENT, || {
            build_oauth_http_client(OAuthHttpClientConfig {
                allow_private_ips: false,
                ..OAuthHttpClientConfig::default()
            })
        })
    }
}

#[cfg(feature = "oidc")]
fn cached_oauth_http_client(
    cache: &'static OnceLock<Result<OAuthHttpClient, String>>,
    build: impl FnOnce() -> Result<OAuthHttpClient, String>,
) -> Result<&'static OAuthHttpClient, RustAuthError> {
    cache
        .get_or_init(build)
        .as_ref()
        .map_err(|message| RustAuthError::OAuth(message.clone()))
}

#[cfg(feature = "oidc")]
fn build_oauth_http_client(config: OAuthHttpClientConfig) -> Result<OAuthHttpClient, String> {
    OAuthHttpClient::from_config(config)
        .map_err(|error| format!("failed to build OIDC HTTP client: {error}"))
}

/// Returns the underlying `reqwest::Client` for the requested SSRF policy,
/// sharing the same guard and pool as [`oauth_http_client`].
#[cfg(feature = "oidc")]
pub(crate) fn http_client(
    allow_private_ips: bool,
) -> Result<&'static reqwest::Client, RustAuthError> {
    Ok(oauth_http_client(allow_private_ips)?.reqwest_client())
}

pub fn safe_redirect_url(context: &AuthContext, value: &str) -> Option<String> {
    if value.is_empty() || value.trim() != value {
        return None;
    }
    let settings = Some(OriginMatchSettings {
        allow_relative_paths: true,
    });
    if !context.is_trusted_origin(value, settings) || is_sso_redirect_loop(value) {
        return None;
    }
    Some(value.to_owned())
}

pub fn constant_time_eq(left: &str, right: &str) -> bool {
    left.as_bytes().ct_eq(right.as_bytes()).into()
}

#[cfg(feature = "saml")]
fn sha256_hex(value: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    hex::encode(hasher.finalize())
}

#[cfg(feature = "saml")]
fn normalize_certificate(cert: &str) -> String {
    cert.lines()
        .filter(|line| !line.starts_with("-----BEGIN ") && !line.starts_with("-----END "))
        .flat_map(|line| line.chars())
        .filter(|character| !character.is_whitespace())
        .collect()
}

#[cfg(feature = "saml")]
fn public_key_algorithm(public_key: PublicKey<'_>) -> String {
    match public_key {
        PublicKey::RSA(key) => format!("RSA-{}", key.key_size()),
        PublicKey::EC(key) => format!("EC-{}", key.key_size()),
        PublicKey::DSA(key) => format!("DSA-{}", key.len() * 8),
        PublicKey::GostR3410(key) => format!("GOST-R3410-{}", key.len() * 8),
        PublicKey::GostR3410_2012(key) => format!("GOST-R3410-2012-{}", key.len() * 8),
        PublicKey::Unknown(_) => "unknown".to_owned(),
    }
}

fn is_sso_redirect_loop(value: &str) -> bool {
    let path = if value.starts_with('/') {
        value.split('?').next().unwrap_or(value).to_owned()
    } else {
        match url::Url::parse(value) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => url.path().to_owned(),
            _ => return true,
        }
    };
    let path = path.trim_end_matches('/');
    path == "/sign-in/sso"
        || path == "/sso/callback"
        || path.starts_with("/sso/callback/")
        || path == "/sso/saml2/callback"
        || path.starts_with("/sso/saml2/callback/")
        || path == "/sso/saml2/sp/acs"
        || path.starts_with("/sso/saml2/sp/acs/")
}

#[cfg(all(test, feature = "oidc"))]
mod tests {
    use super::*;

    #[test]
    fn oidc_http_client_build_failure_is_reported() -> Result<(), Box<dyn std::error::Error>> {
        let Err(error) = build_oauth_http_client(OAuthHttpClientConfig {
            timeout: std::time::Duration::ZERO,
            ..OAuthHttpClientConfig::default()
        }) else {
            return Err("expected invalid HTTP client config to fail".into());
        };

        assert!(error.contains("failed to build OIDC HTTP client"));
        Ok(())
    }

    #[test]
    fn cached_oauth_http_client_failure_is_fail_closed() -> Result<(), Box<dyn std::error::Error>> {
        static CLIENT: OnceLock<Result<OAuthHttpClient, String>> = OnceLock::new();

        let Err(error) =
            cached_oauth_http_client(&CLIENT, || Err("guarded client unavailable".to_owned()))
        else {
            return Err("expected cached client failure to be returned".into());
        };

        assert_eq!(
            error,
            RustAuthError::OAuth("guarded client unavailable".to_owned())
        );
        Ok(())
    }
}
