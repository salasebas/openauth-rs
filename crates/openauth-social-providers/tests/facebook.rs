use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use openauth_oauth::oauth2::{ClientId, OAuth2Tokens, ProviderOptions};
use openauth_social_providers::facebook::{
    FacebookOptions, FacebookPicture, FacebookPictureData, FacebookProfile, FacebookProvider,
};
use serde_json::json;

#[test]
fn facebook_authorization_url_uses_upstream_defaults() -> Result<(), Box<dyn std::error::Error>> {
    let provider = FacebookProvider::new(FacebookOptions {
        oauth: provider_options(),
        config_id: Some("login-config".to_owned()),
        ..FacebookOptions::default()
    });

    let url = provider.create_authorization_url(
        "state-value",
        ["business_management".to_owned()],
        "https://app.example.com/callback",
        Some("user@example.com"),
    )?;

    assert_eq!(
        url.as_str().split('?').next(),
        Some("https://www.facebook.com/v24.0/dialog/oauth")
    );
    assert_eq!(query(&url, "response_type"), Some("code".to_owned()));
    assert_eq!(query(&url, "client_id"), Some("fb-web".to_owned()));
    assert_eq!(
        query(&url, "redirect_uri"),
        Some("https://app.example.com/callback".to_owned())
    );
    assert_eq!(query(&url, "state"), Some("state-value".to_owned()));
    assert_eq!(
        query(&url, "scope"),
        Some("email public_profile pages_show_list business_management".to_owned())
    );
    assert_eq!(
        query(&url, "login_hint"),
        Some("user@example.com".to_owned())
    );
    assert_eq!(query(&url, "config_id"), Some("login-config".to_owned()));
    Ok(())
}

#[test]
fn facebook_authorization_url_rejects_missing_required_credentials() {
    let provider = FacebookProvider::new(FacebookOptions::default());

    let result = provider.create_authorization_url(
        "state",
        Vec::<String>::new(),
        "https://app/callback",
        None,
    );

    assert!(result.is_err());
}

#[test]
fn facebook_authorization_url_can_disable_default_scopes() -> Result<(), Box<dyn std::error::Error>>
{
    let provider = FacebookProvider::new(FacebookOptions {
        oauth: ProviderOptions {
            disable_default_scope: true,
            ..provider_options()
        },
        ..FacebookOptions::default()
    });

    let url = provider.create_authorization_url(
        "state",
        ["custom_scope".to_owned()],
        "https://app.example.com/callback",
        None,
    )?;

    assert_eq!(
        query(&url, "scope"),
        Some("pages_show_list custom_scope".to_owned())
    );
    Ok(())
}

#[test]
fn facebook_profile_mapping_matches_graph_profile_behavior() {
    let provider = FacebookProvider::new(FacebookOptions::default());
    let profile = FacebookProfile {
        id: "123".to_owned(),
        name: "Ada Lovelace".to_owned(),
        email: Some("ada@example.com".to_owned()),
        email_verified: None,
        picture: FacebookPicture {
            data: FacebookPictureData {
                height: 100,
                is_silhouette: false,
                url: "https://cdn.example.com/ada.png".to_owned(),
                width: 100,
            },
        },
    };

    let info = provider.user_info_from_profile(profile);

    assert_eq!(info.user.id, "123");
    assert_eq!(info.user.name.as_deref(), Some("Ada Lovelace"));
    assert_eq!(info.user.email.as_deref(), Some("ada@example.com"));
    assert_eq!(
        info.user.image.as_deref(),
        Some("https://cdn.example.com/ada.png")
    );
    assert!(!info.user.email_verified);
}

#[test]
fn facebook_limited_login_id_token_maps_to_user_with_unverified_email(
) -> Result<(), Box<dyn std::error::Error>> {
    let provider = FacebookProvider::new(FacebookOptions::default());
    let token = unsigned_jwt(json!({
        "sub": "limited-user",
        "email": "limited@example.com",
        "name": "Limited User",
        "picture": "https://cdn.example.com/limited.png"
    }))?;

    let info = provider
        .user_info_from_id_token(&token)?
        .ok_or("valid id token payload")?;

    assert_eq!(info.user.id, "limited-user");
    assert_eq!(info.user.email.as_deref(), Some("limited@example.com"));
    assert_eq!(
        info.user.image.as_deref(),
        Some("https://cdn.example.com/limited.png")
    );
    assert!(!info.user.email_verified);
    Ok(())
}

#[test]
fn facebook_user_info_url_extends_default_fields() -> Result<(), Box<dyn std::error::Error>> {
    let provider = FacebookProvider::new(FacebookOptions {
        fields: vec!["first_name".to_owned(), "last_name".to_owned()],
        ..FacebookOptions::default()
    });

    let url = provider.user_info_url()?;

    assert_eq!(
        query(&url, "fields"),
        Some("id,name,email,picture,first_name,last_name".to_owned())
    );
    Ok(())
}

#[tokio::test]
async fn facebook_verify_id_token_accepts_opaque_access_tokens() {
    let provider = FacebookProvider::new(FacebookOptions {
        oauth: provider_options(),
        ..FacebookOptions::default()
    });

    assert!(provider.verify_id_token("opaque-token", None).await);
}

#[tokio::test]
async fn facebook_get_user_info_returns_none_when_access_token_is_missing(
) -> Result<(), Box<dyn std::error::Error>> {
    let provider = FacebookProvider::new(FacebookOptions::default());
    let tokens = OAuth2Tokens::default();

    let info = provider.get_user_info(&tokens).await?;

    assert!(info.is_none());
    Ok(())
}

fn provider_options() -> ProviderOptions {
    ProviderOptions {
        client_id: Some(ClientId::Multiple(vec![
            "fb-web".to_owned(),
            "fb-mobile".to_owned(),
        ])),
        client_secret: Some("fb-secret".to_owned()),
        scope: vec!["pages_show_list".to_owned()],
        ..ProviderOptions::default()
    }
}

fn query(url: &url::Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

fn unsigned_jwt(payload: serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
    let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&json!({ "alg": "none" }))?);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload)?);
    Ok(format!("{header}.{payload}."))
}
