#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OAuthUserInfoError {
    AccountNotLinked,
    SignupDisabled,
    UnableToCreateUser,
    UnableToCreateSession,
    UnableToLinkAccount,
}

const HANDLING_DOCS_URL: &str =
    "https://www.better-auth.com/docs/concepts/oauth#handling-providers-without-email";

pub fn missing_email_log_message(provider_id: &str, source: Option<&str>) -> String {
    let subject = if source == Some("generic") {
        format!("Generic OAuth provider \"{provider_id}\"")
    } else {
        format!("Provider \"{provider_id}\"")
    };
    let where_text = if source == Some("id_token") {
        " in the id token"
    } else {
        ""
    };
    format!(
        "{subject} did not return an email{where_text}. Either request the provider's email scope, or synthesize one via `mapProfileToUser`. See {HANDLING_DOCS_URL}"
    )
}
