#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "plugin tests intentionally fail fast with contextual setup errors"
)]

use http::{header, Method, Request, Response, StatusCode};
use openauth_core::api::AuthRouter;
use openauth_core::context::{create_auth_context_with_adapter, AuthContext};
use openauth_core::cookies::{
    get_session_cookie, set_session_cookie, verify_cookie_value, Cookie, SessionCookieOptions,
};
use openauth_core::db::{DbAdapter, MemoryAdapter};
use openauth_core::options::{AdvancedOptions, OpenAuthOptions};
use openauth_core::plugin::AuthPlugin;
use openauth_core::session::{CreateSessionInput, DbSessionStore};
use openauth_core::user::{CreateOAuthAccountInput, CreateUserInput, DbUserStore};
use openauth_oauth::oauth2::{
    ClientAuthentication, OAuth2Tokens, OAuth2UserInfo, OAuthError, SocialAuthorizationCodeRequest,
    SocialAuthorizationUrlRequest, SocialOAuthProvider,
};
use openauth_plugins::generic_oauth::{
    auth0, generic_oauth, gumroad, hubspot, keycloak, line, microsoft_entra_id, okta, patreon,
    slack, Auth0Options, BaseOAuthProviderOptions, GenericOAuthConfig, GenericOAuthOptions,
    GenericOAuthTokenRequest, GumroadOptions, HubSpotOptions, KeycloakOptions, LineOptions,
    MicrosoftEntraIdOptions, OktaOptions, PatreonOptions, SlackOptions, UPSTREAM_PLUGIN_ID,
};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use time::{Duration, OffsetDateTime};

#[test]
fn generic_oauth_plugin_exposes_metadata_endpoints_and_errors() {
    let plugin = generic_oauth(GenericOAuthOptions {
        config: vec![example_config()],
    });

    assert_eq!(plugin.id, UPSTREAM_PLUGIN_ID);
    assert_eq!(plugin.version.as_deref(), Some(openauth_plugins::VERSION));
    assert_eq!(plugin.endpoints.len(), 3);
    assert!(plugin
        .error_codes
        .iter()
        .any(|code| code.code == "ISSUER_MISMATCH"));
}

#[test]
fn generic_oauth_init_registers_configured_social_providers() {
    let plugin = generic_oauth(GenericOAuthOptions {
        config: vec![example_config()],
    });
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("https://app.example.com".to_owned()),
            plugins: vec![plugin.clone()],
            ..OpenAuthOptions::default()
        },
        Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>,
    )
    .unwrap();

    assert!(context.social_provider("example").is_some());
}

#[test]
fn generic_oauth_duplicate_provider_ids_keep_first_provider() {
    let mut duplicate = example_config();
    duplicate.authorization_url = Some("https://other.example.com/oauth/authorize".to_owned());
    let plugin = generic_oauth(GenericOAuthOptions {
        config: vec![example_config(), duplicate],
    });
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("https://app.example.com".to_owned()),
            plugins: vec![plugin.clone()],
            ..OpenAuthOptions::default()
        },
        Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>,
    )
    .unwrap();

    assert!(context.social_provider("example").is_some());
}

#[test]
fn provider_authorization_url_uses_better_auth_oauth2_callback_and_pkce() -> Result<(), OAuthError>
{
    let provider = provider(example_config());
    let url = provider.create_authorization_url(SocialAuthorizationUrlRequest {
        state: "state-1".to_owned(),
        redirect_uri: "https://app.example.com/oauth2/callback/example".to_owned(),
        code_verifier: Some("01234567890123456789012345678901234567890123456789".to_owned()),
        scopes: vec!["calendar".to_owned()],
        login_hint: Some("ada@example.com".to_owned()),
    })?;

    assert_eq!(
        url.as_str().split('?').next(),
        Some("https://idp.example.com/oauth/authorize")
    );
    assert_eq!(query_value(&url, "client_id"), Some("client-1".to_owned()));
    assert_eq!(query_value(&url, "state"), Some("state-1".to_owned()));
    assert_eq!(
        query_value(&url, "redirect_uri"),
        Some("https://app.example.com/oauth2/callback/example".to_owned())
    );
    assert_eq!(
        query_value(&url, "scope"),
        Some("calendar openid email".to_owned())
    );
    assert_eq!(query_value(&url, "prompt"), Some("consent".to_owned()));
    assert_eq!(
        query_value(&url, "code_challenge_method"),
        Some("S256".to_owned())
    );
    assert_eq!(query_value(&url, "audience"), Some("api".to_owned()));
    Ok(())
}

#[test]
fn provider_authorization_code_request_uses_basic_auth_and_extra_params() -> Result<(), OAuthError>
{
    let mut config = example_config();
    config.authentication = ClientAuthentication::Basic;
    config
        .token_url_params
        .insert("resource".to_owned(), "https://api.example.com".to_owned());
    let provider = provider(config);
    let request = provider.authorization_code_request(SocialAuthorizationCodeRequest {
        code: "code-1".to_owned(),
        code_verifier: Some("verifier-1".to_owned()),
        redirect_uri: "https://app.example.com/oauth2/callback/example".to_owned(),
        device_id: None,
    })?;

    assert_eq!(request.form_value("grant_type"), Some("authorization_code"));
    assert_eq!(request.form_value("code"), Some("code-1"));
    assert_eq!(
        request.form_value("resource"),
        Some("https://api.example.com")
    );
    assert!(request.header("authorization").is_some());
    assert_eq!(request.form_value("client_secret"), None);
    Ok(())
}

#[tokio::test]
async fn provider_uses_custom_get_token_and_maps_profile() {
    let mut config = example_config();
    config.get_token = Some(Arc::new(|request: GenericOAuthTokenRequest| {
        Box::pin(async move {
            assert_eq!(request.code, "code-1");
            assert_eq!(
                request.redirect_uri,
                "https://app.example.com/oauth2/callback/example"
            );
            Ok(OAuth2Tokens {
                access_token: Some("access-1".to_owned()),
                id_token: Some(jwt_with_claims(
                    r#"{"sub":123,"email":"ada@example.com","name":"Ada"}"#,
                )),
                ..OAuth2Tokens::default()
            })
        })
    }));
    config.map_profile_to_user = Some(Arc::new(|mut profile: OAuth2UserInfo| {
        Box::pin(async move {
            profile.id = format!("mapped-{}", profile.id);
            profile.name = Some("Ada Lovelace".to_owned());
            profile.email_verified = true;
            Ok(profile)
        })
    }));
    let provider = provider(config);
    let tokens = provider
        .validate_authorization_code(SocialAuthorizationCodeRequest {
            code: "code-1".to_owned(),
            code_verifier: None,
            redirect_uri: "https://app.example.com/oauth2/callback/example".to_owned(),
            device_id: None,
        })
        .await
        .unwrap();
    let user = provider.get_user_info(tokens, None).await.unwrap().unwrap();

    assert_eq!(user.id, "mapped-123");
    assert_eq!(user.name.as_deref(), Some("Ada Lovelace"));
    assert!(user.email_verified);
}

#[test]
fn helper_providers_match_upstream_defaults() {
    assert_eq!(
        auth0(Auth0Options {
            base: helper_base("client", "secret"),
            domain: "https://tenant.auth0.com".to_owned(),
        })
        .discovery_url,
        Some("https://tenant.auth0.com/.well-known/openid-configuration".to_owned())
    );
    assert_eq!(
        okta(OktaOptions {
            base: helper_base("client", "secret"),
            issuer: "https://dev.okta.com/oauth2/default/".to_owned(),
        })
        .discovery_url,
        Some("https://dev.okta.com/oauth2/default/.well-known/openid-configuration".to_owned())
    );
    assert_eq!(
        keycloak(KeycloakOptions {
            base: helper_base("client", "secret"),
            issuer: "https://kc.example.com/realms/acme/".to_owned(),
        })
        .discovery_url,
        Some("https://kc.example.com/realms/acme/.well-known/openid-configuration".to_owned())
    );
    assert_eq!(
        gumroad(GumroadOptions {
            base: helper_base("client", "secret"),
        })
        .provider_id,
        "gumroad"
    );
    assert_eq!(
        hubspot(HubSpotOptions {
            base: helper_base("client", "secret"),
        })
        .scopes,
        vec!["oauth"]
    );
    assert_eq!(
        line(LineOptions {
            base: helper_base("client", "secret"),
            provider_id: Some("line-jp".to_owned()),
        })
        .provider_id,
        "line-jp"
    );
    assert_eq!(
        microsoft_entra_id(MicrosoftEntraIdOptions {
            base: helper_base("client", "secret"),
            tenant_id: "common".to_owned(),
        })
        .authorization_url,
        Some("https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_owned())
    );
    assert_eq!(
        patreon(PatreonOptions {
            base: helper_base("client", "secret"),
        })
        .scopes,
        vec!["identity[email]"]
    );
    assert_eq!(
        slack(SlackOptions {
            base: helper_base("client", "secret"),
        })
        .provider_id,
        "slack"
    );
}

#[test]
fn helper_provider_options_apply_overrides() {
    let config = slack(SlackOptions {
        base: BaseOAuthProviderOptions {
            client_id: "client".to_owned(),
            client_secret: Some("secret".to_owned()),
            scopes: Some(vec!["openid".to_owned(), "team".to_owned()]),
            redirect_uri: Some("https://app.example.com/custom/callback".to_owned()),
            pkce: true,
            disable_implicit_sign_up: true,
            disable_sign_up: true,
            override_user_info: true,
        },
    });

    assert_eq!(config.scopes, vec!["openid", "team"]);
    assert_eq!(
        config.redirect_uri.as_deref(),
        Some("https://app.example.com/custom/callback")
    );
    assert!(config.pkce);
    assert!(config.disable_implicit_sign_up);
    assert!(config.disable_sign_up);
    assert!(config.override_user_info);
}

#[tokio::test]
async fn sign_in_oauth2_route_returns_redirect_url() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let plugin = generic_oauth(GenericOAuthOptions {
        config: vec![example_config()],
    });
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("https://app.example.com".to_owned()),
            plugins: vec![plugin],
            ..OpenAuthOptions::default()
        },
        adapter,
    )
    .unwrap();
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let response = router
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("https://app.example.com/api/auth/sign-in/oauth2")
                .header("content-type", "application/json")
                .body(
                    br#"{"providerId":"example","callbackURL":"/dashboard","disableRedirect":true}"#
                        .to_vec(),
                )
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(response.body()).unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body["redirect"], false);
    let url = url::Url::parse(body["url"].as_str().unwrap()).unwrap();
    assert_eq!(
        query_value(&url, "redirect_uri"),
        Some("https://app.example.com/oauth2/callback/example".to_owned())
    );
}

#[tokio::test]
async fn sign_in_oauth2_route_rejects_unknown_provider() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let plugin = generic_oauth(GenericOAuthOptions {
        config: vec![example_config()],
    });
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("https://app.example.com".to_owned()),
            plugins: vec![plugin],
            ..OpenAuthOptions::default()
        },
        adapter,
    )
    .unwrap();
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let response = router
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("https://app.example.com/api/auth/sign-in/oauth2")
                .header("content-type", "application/json")
                .body(br#"{"providerId":"missing"}"#.to_vec())
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(response.body()).unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "PROVIDER_CONFIG_NOT_FOUND");
}

#[tokio::test]
async fn oauth2_callback_creates_user_account_session_and_cookie() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let context = context_with_plugin(
        adapter.clone(),
        oauth_plugin(oauth_flow_config("oauth-user-1")),
    );
    let router = AuthRouter::try_new(context.clone(), Vec::new()).unwrap();
    let state = sign_in_state(&router, "example", "/dashboard", None, false)
        .await
        .unwrap();

    let response = oauth_callback(&router, "example", "code-1", &state)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/dashboard"));
    let user = DbUserStore::new(adapter.as_ref())
        .find_user_by_email("ada@example.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.name, "Ada Lovelace");
    assert!(DbUserStore::new(adapter.as_ref())
        .find_account_by_provider_account("oauth-user-1", "example")
        .await
        .unwrap()
        .is_some());
    let token = session_token_from_response(&context, &response);
    assert!(DbSessionStore::new(adapter.as_ref())
        .find_session(&token)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn oauth2_callback_redirects_new_user_to_new_user_callback_url() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let context = context_with_plugin(adapter, oauth_plugin(oauth_flow_config("oauth-user-2")));
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let state = sign_in_state(&router, "example", "/dashboard", Some("/welcome"), false)
        .await
        .unwrap();

    let response = oauth_callback(&router, "example", "code-1", &state)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/welcome"));
}

#[tokio::test]
async fn oauth2_callback_redirects_signup_disabled_error() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let mut config = oauth_flow_config("oauth-user-3");
    config.disable_implicit_sign_up = true;
    let context = context_with_plugin(adapter, oauth_plugin(config));
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let state = sign_in_state(&router, "example", "/dashboard", None, false)
        .await
        .unwrap();

    let response = oauth_callback(&router, "example", "code-1", &state)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        location(&response),
        Some("https://app.example.com/error?error=signup_disabled")
    );
}

#[tokio::test]
async fn oauth2_callback_allows_request_signup_when_implicit_signup_is_disabled() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let mut config = oauth_flow_config("oauth-user-4");
    config.disable_implicit_sign_up = true;
    let context = context_with_plugin(adapter, oauth_plugin(config));
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let state = sign_in_state(&router, "example", "/dashboard", None, true)
        .await
        .unwrap();

    let response = oauth_callback(&router, "example", "code-1", &state)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(location(&response), Some("/dashboard"));
}

#[tokio::test]
async fn oauth2_callback_uses_custom_redirect_uri_in_token_exchange() {
    let seen = Arc::new(std::sync::Mutex::new(String::new()));
    let mut config = oauth_flow_config("oauth-user-5");
    config.redirect_uri = Some("https://app.example.com/custom/oauth/callback".to_owned());
    config.get_token = Some({
        let seen = Arc::clone(&seen);
        Arc::new(move |request: GenericOAuthTokenRequest| {
            let seen = Arc::clone(&seen);
            Box::pin(async move {
                *seen.lock().unwrap() = request.redirect_uri;
                Ok(OAuth2Tokens {
                    access_token: Some("access-token".to_owned()),
                    ..OAuth2Tokens::default()
                })
            })
        })
    });
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let context = context_with_plugin(adapter, oauth_plugin(config));
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let sign_in = sign_in_url(&router, "example", "/dashboard", None, false)
        .await
        .unwrap();
    assert_eq!(
        query_value(&sign_in, "redirect_uri"),
        Some("https://app.example.com/custom/oauth/callback".to_owned())
    );
    let state = query_value(&sign_in, "state").unwrap();

    let response = oauth_callback(&router, "example", "code-1", &state)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        seen.lock().unwrap().as_str(),
        "https://app.example.com/custom/oauth/callback"
    );
}

#[tokio::test]
async fn oauth2_callback_rejects_missing_state() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let context = context_with_plugin(adapter, oauth_plugin(oauth_flow_config("oauth-user-6")));
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();

    let response = router
        .handle_async(
            Request::builder()
                .method(Method::GET)
                .uri("https://app.example.com/api/auth/oauth2/callback/example?code=code-1")
                .body(Vec::new())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        location(&response),
        Some("https://app.example.com/error?error=invalid_state")
    );
}

#[tokio::test]
async fn sign_in_oauth2_caches_discovery_by_provider() {
    let hits = Arc::new(AtomicUsize::new(0));
    let discovery_url = discovery_server(Arc::clone(&hits));
    let mut config =
        GenericOAuthConfig::discovery("discovery", "client-1", Some("secret-1"), discovery_url);
    config.scopes = vec!["openid".to_owned()];
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let plugin = generic_oauth(GenericOAuthOptions {
        config: vec![config],
    });
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("https://app.example.com".to_owned()),
            plugins: vec![plugin],
            ..OpenAuthOptions::default()
        },
        adapter,
    )
    .unwrap();
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();

    for _ in 0..2 {
        let response = router
            .handle_async(
                Request::builder()
                    .method(Method::POST)
                    .uri("https://app.example.com/api/auth/sign-in/oauth2")
                    .header("content-type", "application/json")
                    .body(br#"{"providerId":"discovery","disableRedirect":true}"#.to_vec())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn oauth2_callback_rejects_issuer_mismatch() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let mut config = example_config();
    config.issuer = Some("https://issuer.example.com".to_owned());
    config.require_issuer_validation = true;
    let plugin = generic_oauth(GenericOAuthOptions {
        config: vec![config],
    });
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("https://app.example.com".to_owned()),
            plugins: vec![plugin],
            ..OpenAuthOptions::default()
        },
        adapter,
    )
    .unwrap();
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let sign_in = router
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("https://app.example.com/api/auth/sign-in/oauth2")
                .header("content-type", "application/json")
                .body(
                    br#"{"providerId":"example","callbackURL":"/dashboard","errorCallbackURL":"/oauth-error","disableRedirect":true}"#
                        .to_vec(),
                )
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(sign_in.body()).unwrap();
    let auth_url = url::Url::parse(body["url"].as_str().unwrap()).unwrap();
    let state = query_value(&auth_url, "state").unwrap();
    let response = router
        .handle_async(
            Request::builder()
                .method(Method::GET)
                .uri(format!("https://app.example.com/api/auth/oauth2/callback/example?code=code-1&state={state}&iss=https%3A%2F%2Fwrong.example.com"))
                .body(Vec::new())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok()),
        Some("/oauth-error?error=issuer_mismatch")
    );
}

#[tokio::test]
async fn oauth2_link_requires_session() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    let plugin = generic_oauth(GenericOAuthOptions {
        config: vec![example_config()],
    });
    let context = create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("https://app.example.com".to_owned()),
            plugins: vec![plugin],
            ..OpenAuthOptions::default()
        },
        adapter,
    )
    .unwrap();
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let response = router
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("https://app.example.com/api/auth/oauth2/link")
                .header("content-type", "application/json")
                .body(br#"{"providerId":"example","callbackURL":"/settings"}"#.to_vec())
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(response.body()).unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "SESSION_REQUIRED");
}

#[tokio::test]
async fn oauth2_link_creates_account_for_current_user() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    seed_user(adapter.as_ref(), "user_1", "ada@example.com").await;
    let context = context_with_plugin(adapter.clone(), oauth_plugin(oauth_flow_config("linked-1")));
    let cookie = session_cookie_for(adapter.as_ref(), &context, "user_1").await;
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let state = link_state(&router, "example", &cookie).await.unwrap();

    let response = oauth_callback(&router, "example", "code-1", &state)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert!(DbUserStore::new(adapter.as_ref())
        .find_account_by_provider_account("linked-1", "example")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn oauth2_link_rejects_account_owned_by_different_user() {
    let adapter = Arc::new(MemoryAdapter::new()) as Arc<dyn DbAdapter>;
    seed_user(adapter.as_ref(), "user_1", "ada@example.com").await;
    seed_user(adapter.as_ref(), "user_2", "grace@example.com").await;
    DbUserStore::new(adapter.as_ref())
        .link_account(CreateOAuthAccountInput {
            id: None,
            provider_id: "example".to_owned(),
            account_id: "linked-2".to_owned(),
            user_id: "user_2".to_owned(),
            access_token: Some("old-token".to_owned()),
            refresh_token: None,
            id_token: None,
            access_token_expires_at: None,
            refresh_token_expires_at: None,
            scope: None,
        })
        .await
        .unwrap();
    let context = context_with_plugin(adapter.clone(), oauth_plugin(oauth_flow_config("linked-2")));
    let cookie = session_cookie_for(adapter.as_ref(), &context, "user_1").await;
    let router = AuthRouter::try_new(context, Vec::new()).unwrap();
    let state = link_state(&router, "example", &cookie).await.unwrap();

    let response = oauth_callback(&router, "example", "code-1", &state)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        location(&response),
        Some("https://app.example.com/error?error=account_already_linked_to_different_user")
    );
}

fn example_config() -> GenericOAuthConfig {
    let mut config = GenericOAuthConfig::new(
        "example",
        "client-1",
        Some("secret-1"),
        "https://idp.example.com/oauth/authorize",
        "https://idp.example.com/oauth/token",
    );
    config.user_info_url = Some("https://idp.example.com/oauth/userinfo".to_owned());
    config.scopes = vec!["openid".to_owned(), "email".to_owned()];
    config.pkce = true;
    config.prompt = Some("consent".to_owned());
    config
        .authorization_url_params
        .insert("audience".to_owned(), "api".to_owned());
    config
}

fn provider(config: GenericOAuthConfig) -> openauth_plugins::generic_oauth::GenericOAuthProvider {
    openauth_plugins::generic_oauth::GenericOAuthProvider::new(config)
}

fn helper_base(client_id: &str, client_secret: &str) -> BaseOAuthProviderOptions {
    BaseOAuthProviderOptions {
        client_id: client_id.to_owned(),
        client_secret: Some(client_secret.to_owned()),
        ..BaseOAuthProviderOptions::default()
    }
}

fn query_value(url: &url::Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

fn discovery_server(hits: Arc<AtomicUsize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let mut stream = stream.unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            hits.fetch_add(1, Ordering::SeqCst);
            let body = r#"{"authorization_endpoint":"https://idp.example.com/oauth/authorize","token_endpoint":"https://idp.example.com/oauth/token","userinfo_endpoint":"https://idp.example.com/oauth/userinfo","issuer":"https://idp.example.com"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    format!("http://{address}/.well-known/openid-configuration")
}

fn oauth_flow_config(user_id: &str) -> GenericOAuthConfig {
    let mut config = example_config();
    let user_id = user_id.to_owned();
    config.get_token = Some(Arc::new(|_request| {
        Box::pin(async {
            Ok(OAuth2Tokens {
                access_token: Some("access-token".to_owned()),
                refresh_token: Some("refresh-token".to_owned()),
                scopes: vec!["openid".to_owned(), "email".to_owned()],
                ..OAuth2Tokens::default()
            })
        })
    }));
    config.get_user_info = Some(Arc::new(move |_tokens| {
        let user_id = user_id.clone();
        Box::pin(async move {
            Ok(Some(OAuth2UserInfo {
                id: user_id,
                name: Some("Ada Lovelace".to_owned()),
                email: Some("ada@example.com".to_owned()),
                image: Some("https://img.example.com/ada.png".to_owned()),
                email_verified: true,
            }))
        })
    }));
    config
}

fn oauth_plugin(config: GenericOAuthConfig) -> AuthPlugin {
    generic_oauth(GenericOAuthOptions {
        config: vec![config],
    })
}

fn context_with_plugin(adapter: Arc<dyn DbAdapter>, plugin: AuthPlugin) -> AuthContext {
    create_auth_context_with_adapter(
        OpenAuthOptions {
            base_url: Some("https://app.example.com".to_owned()),
            secret: Some(secret().to_owned()),
            plugins: vec![plugin],
            advanced: AdvancedOptions {
                disable_csrf_check: true,
                disable_origin_check: true,
                ..AdvancedOptions::default()
            },
            ..OpenAuthOptions::default()
        },
        adapter,
    )
    .unwrap()
}

async fn sign_in_url(
    router: &AuthRouter,
    provider_id: &str,
    callback_url: &str,
    new_user_url: Option<&str>,
    request_sign_up: bool,
) -> Result<url::Url, Box<dyn std::error::Error>> {
    let new_user = new_user_url
        .map(|url| format!(r#","newUserCallbackURL":"{url}""#))
        .unwrap_or_default();
    let request_sign_up = if request_sign_up {
        r#","requestSignUp":true"#
    } else {
        ""
    };
    let response = router
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("https://app.example.com/api/auth/sign-in/oauth2")
                .header(header::CONTENT_TYPE, "application/json")
                .body(
                    format!(
                        r#"{{"providerId":"{provider_id}","callbackURL":"{callback_url}","disableRedirect":true{new_user}{request_sign_up}}}"#
                    )
                    .into_bytes(),
                )?,
        )
        .await?;
    let body: Value = serde_json::from_slice(response.body())?;
    Ok(url::Url::parse(body["url"].as_str().ok_or("missing url")?)?)
}

async fn sign_in_state(
    router: &AuthRouter,
    provider_id: &str,
    callback_url: &str,
    new_user_url: Option<&str>,
    request_sign_up: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = sign_in_url(
        router,
        provider_id,
        callback_url,
        new_user_url,
        request_sign_up,
    )
    .await?;
    query_value(&url, "state").ok_or_else(|| "missing state".into())
}

async fn link_state(
    router: &AuthRouter,
    provider_id: &str,
    cookie: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = router
        .handle_async(
            Request::builder()
                .method(Method::POST)
                .uri("https://app.example.com/api/auth/oauth2/link")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(
                    format!(r#"{{"providerId":"{provider_id}","callbackURL":"/settings"}}"#)
                        .into_bytes(),
                )?,
        )
        .await?;
    let body: Value = serde_json::from_slice(response.body())?;
    let url = url::Url::parse(body["url"].as_str().ok_or_else(|| {
        format!(
            "missing url in {} response: {}",
            response.status(),
            String::from_utf8_lossy(response.body())
        )
    })?)?;
    query_value(&url, "state").ok_or_else(|| "missing state".into())
}

async fn oauth_callback(
    router: &AuthRouter,
    provider_id: &str,
    code: &str,
    state: &str,
) -> Result<Response<Vec<u8>>, openauth_core::error::OpenAuthError> {
    router
        .handle_async(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "https://app.example.com/api/auth/oauth2/callback/{provider_id}?code={code}&state={state}"
                ))
                .body(Vec::new())
                .unwrap(),
        )
        .await
}

fn location(response: &Response<Vec<u8>>) -> Option<&str> {
    response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
}

fn session_token_from_response(context: &AuthContext, response: &Response<Vec<u8>>) -> String {
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .unwrap();
    let signed = get_session_cookie(cookie, None, None).unwrap();
    verify_cookie_value(&signed, &context.secret)
        .unwrap()
        .unwrap()
}

async fn seed_user(adapter: &dyn DbAdapter, id: &str, email: &str) {
    DbUserStore::new(adapter)
        .create_user(CreateUserInput::new("Ada Lovelace", email).id(id))
        .await
        .unwrap();
}

async fn session_cookie_for(
    adapter: &dyn DbAdapter,
    context: &AuthContext,
    user_id: &str,
) -> String {
    let session = DbSessionStore::new(adapter)
        .create_session(CreateSessionInput::new(
            user_id,
            OffsetDateTime::now_utc() + Duration::hours(1),
        ))
        .await
        .unwrap();
    cookie_header(
        &set_session_cookie(
            &context.auth_cookies,
            &context.secret,
            &session.token,
            SessionCookieOptions::default(),
        )
        .unwrap(),
    )
}

fn cookie_header(cookies: &[Cookie]) -> String {
    cookies
        .iter()
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ")
}

fn secret() -> &'static str {
    "test-secret-123456789012345678901234"
}

fn jwt_with_claims(claims: &str) -> String {
    fn encode(input: &str) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let bytes = input.as_bytes();
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            output.push(TABLE[(b0 >> 2) as usize] as char);
            output.push(TABLE[(((b0 & 0b11) << 4) | (b1 >> 4)) as usize] as char);
            if chunk.len() > 1 {
                output.push(TABLE[(((b1 & 0b1111) << 2) | (b2 >> 6)) as usize] as char);
            }
            if chunk.len() > 2 {
                output.push(TABLE[(b2 & 0b111111) as usize] as char);
            }
        }
        output
    }

    format!("{}.{}.", encode(r#"{"alg":"none"}"#), encode(claims))
}
