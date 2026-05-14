use openauth_oauth::oauth2::{
    ClientId, ProviderOptions, SocialAuthorizationUrlRequest, SocialOAuthProvider,
};
use openauth_social_providers::PROVIDER_IDS;
use openauth_social_providers::{
    apple::AppleProvider, atlassian::AtlassianProvider, cognito::CognitoProvider,
    discord::DiscordProvider, dropbox::DropboxProvider, facebook::FacebookProvider,
    figma::FigmaProvider, github::github, github::GitHubProvider, gitlab::GitlabProvider,
    google::google, google::GoogleOptions, google::GoogleProvider,
    huggingface::HuggingFaceProvider, kakao::KakaoProvider, kick::KickProvider, line::LineProvider,
    linear::LinearProvider, linkedin::LinkedInProvider,
    microsoft_entra_id::MicrosoftEntraIdProvider, naver::NaverProvider, notion::NotionProvider,
    paybin::PaybinProvider, paypal::PayPalProvider, polar::PolarProvider, railway::RailwayProvider,
    reddit::RedditProvider, roblox::RobloxProvider, salesforce::SalesforceProvider,
    slack::SlackProvider, spotify::SpotifyProvider, tiktok::TiktokProvider, twitch::TwitchProvider,
    twitter::TwitterProvider, vercel::VercelProvider, vk::VkProvider, wechat::WeChatProvider,
    zoom::ZoomProvider,
};

#[test]
fn social_provider_registry_contains_upstream_provider_names() {
    assert!(PROVIDER_IDS.contains(&"github"));
    assert!(PROVIDER_IDS.contains(&"linkedin"));
    assert!(PROVIDER_IDS.contains(&"microsoft"));
    assert!(PROVIDER_IDS.contains(&"wechat"));
}

#[test]
fn all_provider_types_implement_social_oauth_runtime_trait() {
    fn assert_provider<T: SocialOAuthProvider>() {}

    assert_provider::<AppleProvider>();
    assert_provider::<AtlassianProvider>();
    assert_provider::<CognitoProvider>();
    assert_provider::<DiscordProvider>();
    assert_provider::<DropboxProvider>();
    assert_provider::<FacebookProvider>();
    assert_provider::<FigmaProvider>();
    assert_provider::<GitHubProvider>();
    assert_provider::<GitlabProvider>();
    assert_provider::<GoogleProvider>();
    assert_provider::<HuggingFaceProvider>();
    assert_provider::<KakaoProvider>();
    assert_provider::<KickProvider>();
    assert_provider::<LineProvider>();
    assert_provider::<LinearProvider>();
    assert_provider::<LinkedInProvider>();
    assert_provider::<MicrosoftEntraIdProvider>();
    assert_provider::<NaverProvider>();
    assert_provider::<NotionProvider>();
    assert_provider::<PaybinProvider>();
    assert_provider::<PayPalProvider>();
    assert_provider::<PolarProvider>();
    assert_provider::<RailwayProvider>();
    assert_provider::<RedditProvider>();
    assert_provider::<RobloxProvider>();
    assert_provider::<SalesforceProvider>();
    assert_provider::<SlackProvider>();
    assert_provider::<SpotifyProvider>();
    assert_provider::<TiktokProvider>();
    assert_provider::<TwitchProvider>();
    assert_provider::<TwitterProvider>();
    assert_provider::<VercelProvider>();
    assert_provider::<VkProvider>();
    assert_provider::<WeChatProvider>();
    assert_provider::<ZoomProvider>();
}

#[test]
fn github_runtime_wrapper_exposes_metadata_and_authorization_url(
) -> Result<(), Box<dyn std::error::Error>> {
    let provider = github(provider_options());

    assert_eq!(SocialOAuthProvider::id(&provider), "github");
    assert_eq!(SocialOAuthProvider::name(&provider), "GitHub");
    assert_eq!(
        provider.provider_options().client_id,
        Some(ClientId::Single("client-id".to_owned()))
    );

    let url = SocialOAuthProvider::create_authorization_url(
        &provider,
        SocialAuthorizationUrlRequest {
            state: "state".to_owned(),
            redirect_uri: "https://app.example.com/callback/github".to_owned(),
            ..SocialAuthorizationUrlRequest::default()
        },
    )?;

    assert_eq!(url.host_str(), Some("github.com"));
    assert!(url.as_str().contains("client_id=client-id"));
    Ok(())
}

#[test]
fn google_runtime_wrapper_exposes_metadata_and_authorization_url(
) -> Result<(), Box<dyn std::error::Error>> {
    let provider = google(GoogleOptions {
        oauth: provider_options(),
        ..GoogleOptions::default()
    });

    assert_eq!(SocialOAuthProvider::id(&provider), "google");
    assert_eq!(SocialOAuthProvider::name(&provider), "Google");
    assert_eq!(
        provider.provider_options().client_id,
        Some(ClientId::Single("client-id".to_owned()))
    );

    let url = SocialOAuthProvider::create_authorization_url(
        &provider,
        SocialAuthorizationUrlRequest {
            state: "state".to_owned(),
            redirect_uri: "https://app.example.com/callback/google".to_owned(),
            code_verifier: Some("verifier".to_owned()),
            ..SocialAuthorizationUrlRequest::default()
        },
    )?;

    assert_eq!(url.host_str(), Some("accounts.google.com"));
    assert!(url.as_str().contains("client_id=client-id"));
    Ok(())
}

fn provider_options() -> ProviderOptions {
    ProviderOptions {
        client_id: Some(ClientId::Single("client-id".to_owned())),
        client_secret: Some("client-secret".to_owned()),
        ..ProviderOptions::default()
    }
}
