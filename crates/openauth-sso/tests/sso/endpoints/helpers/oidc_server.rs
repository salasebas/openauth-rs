use josekit::jwk::Jwk;
use josekit::jws::alg::rsassa::RsassaJwsAlgorithm::Rs256;
use josekit::jws::JwsHeader;
use josekit::jwt::{self, JwtPayload};
use serde_json::json;

pub(crate) struct MockOidcServer {
    pub(crate) base_url: String,
    token_requests: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl MockOidcServer {
    pub(crate) async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let base_url = format!("http://{address}");
        let token_requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (valid_id_token, public_jwk) =
            signed_oidc_id_token("client_123456", "https://idp.example.com")?;
        let missing_exp_id_token = signed_oidc_id_token_with_options(
            "client_123456",
            "https://idp.example.com",
            IdTokenOptions {
                include_exp: false,
                ..IdTokenOptions::default()
            },
        )?
        .0;
        let missing_sub_id_token = signed_oidc_id_token_with_options(
            "client_123456",
            "https://idp.example.com",
            IdTokenOptions {
                include_sub: false,
                ..IdTokenOptions::default()
            },
        )?
        .0;
        let (azure_id_token, azure_public_jwk) = signed_oidc_id_token_with_options(
            "client_123456",
            "https://login.microsoftonline.com/11111111-1111-1111-1111-111111111111/v2.0",
            IdTokenOptions::azure(),
        )?;
        let (azure_wrong_issuer_id_token, azure_wrong_issuer_public_jwk) =
            signed_oidc_id_token_with_options(
                "client_123456",
                "https://login.microsoftonline.com/11111111-1111-1111-1111-111111111111/v2.0",
                IdTokenOptions {
                    issuer: Some(
                        "https://login.microsoftonline.com/22222222-2222-2222-2222-222222222222/v2.0"
                            .to_owned(),
                    ),
                    key_id: "azure-wrong-issuer-key".to_owned(),
                    ..IdTokenOptions::azure()
                },
            )?;
        let (multi_audience_missing_azp_id_token, multi_audience_missing_azp_public_jwk) =
            signed_oidc_id_token_with_options(
                "client_123456",
                "https://idp.example.com",
                IdTokenOptions {
                    audience_claim: Some(json!(["client_123456", "secondary-client"])),
                    key_id: "multi-audience-missing-azp-key".to_owned(),
                    ..IdTokenOptions::default()
                },
            )?;
        let (multi_audience_wrong_azp_id_token, multi_audience_wrong_azp_public_jwk) =
            signed_oidc_id_token_with_options(
                "client_123456",
                "https://idp.example.com",
                IdTokenOptions {
                    audience_claim: Some(json!(["client_123456", "secondary-client"])),
                    key_id: "multi-audience-wrong-azp-key".to_owned(),
                    extra_claims: vec![("azp".to_owned(), json!("other-client"))],
                    ..IdTokenOptions::default()
                },
            )?;
        let (multi_audience_valid_azp_id_token, multi_audience_valid_azp_public_jwk) =
            signed_oidc_id_token_with_options(
                "client_123456",
                "https://idp.example.com",
                IdTokenOptions {
                    audience_claim: Some(json!(["client_123456", "secondary-client"])),
                    key_id: "multi-audience-valid-azp-key".to_owned(),
                    extra_claims: vec![("azp".to_owned(), json!("client_123456"))],
                    ..IdTokenOptions::default()
                },
            )?;
        let jwks_body = json!({
            "keys": [
                public_jwk,
                azure_public_jwk,
                azure_wrong_issuer_public_jwk,
                multi_audience_missing_azp_public_jwk,
                multi_audience_wrong_azp_public_jwk,
                multi_audience_valid_azp_public_jwk
            ]
        })
        .to_string();
        let captured_token_requests = std::sync::Arc::clone(&token_requests);
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                let valid_id_token = valid_id_token.clone();
                let jwks_body = jwks_body.clone();
                let missing_exp_id_token = missing_exp_id_token.clone();
                let missing_sub_id_token = missing_sub_id_token.clone();
                let azure_id_token = azure_id_token.clone();
                let azure_wrong_issuer_id_token = azure_wrong_issuer_id_token.clone();
                let multi_audience_missing_azp_id_token =
                    multi_audience_missing_azp_id_token.clone();
                let multi_audience_wrong_azp_id_token = multi_audience_wrong_azp_id_token.clone();
                let multi_audience_valid_azp_id_token = multi_audience_valid_azp_id_token.clone();
                let captured_token_requests = std::sync::Arc::clone(&captured_token_requests);
                tokio::spawn(async move {
                    let mut buffer = [0_u8; 4096];
                    let Ok(read) = tokio::io::AsyncReadExt::read(&mut stream, &mut buffer).await
                    else {
                        return;
                    };
                    let request = String::from_utf8_lossy(&buffer[..read]);
                    if request.starts_with("POST /token ") {
                        if let Ok(mut requests) = captured_token_requests.lock() {
                            requests.push(request.to_string());
                        }
                    }
                    let (status, body) = if request.starts_with("POST /token ")
                        && request.contains("code=id-token-code")
                    {
                        (
                            "200 OK",
                            r#"{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile","id_token":"invalid-id-token"}"#.to_owned(),
                        )
                    } else if request.starts_with("POST /token ")
                        && request.contains("code=valid-id-token-code")
                    {
                        (
                            "200 OK",
                            format!(
                                r#"{{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile","id_token":{}}}"#,
                                serde_json::to_string(&valid_id_token).unwrap_or_default()
                            ),
                        )
                    } else if request.starts_with("POST /token ")
                        && request.contains("code=missing-exp-id-token-code")
                    {
                        (
                            "200 OK",
                            format!(
                                r#"{{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile","id_token":{}}}"#,
                                serde_json::to_string(&missing_exp_id_token).unwrap_or_default()
                            ),
                        )
                    } else if request.starts_with("POST /token ")
                        && request.contains("code=missing-sub-id-token-code")
                    {
                        (
                            "200 OK",
                            format!(
                                r#"{{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile","id_token":{}}}"#,
                                serde_json::to_string(&missing_sub_id_token).unwrap_or_default()
                            ),
                        )
                    } else if request.starts_with("POST /token ")
                        && request.contains("code=azure-id-token-code")
                    {
                        (
                            "200 OK",
                            format!(
                                r#"{{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile","id_token":{}}}"#,
                                serde_json::to_string(&azure_id_token).unwrap_or_default()
                            ),
                        )
                    } else if request.starts_with("POST /token ")
                        && request.contains("code=azure-wrong-issuer-id-token-code")
                    {
                        (
                            "200 OK",
                            format!(
                                r#"{{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile","id_token":{}}}"#,
                                serde_json::to_string(&azure_wrong_issuer_id_token)
                                    .unwrap_or_default()
                            ),
                        )
                    } else if request.starts_with("POST /token ")
                        && request.contains("code=multi-audience-missing-azp-code")
                    {
                        (
                            "200 OK",
                            format!(
                                r#"{{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile","id_token":{}}}"#,
                                serde_json::to_string(&multi_audience_missing_azp_id_token)
                                    .unwrap_or_default()
                            ),
                        )
                    } else if request.starts_with("POST /token ")
                        && request.contains("code=multi-audience-wrong-azp-code")
                    {
                        (
                            "200 OK",
                            format!(
                                r#"{{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile","id_token":{}}}"#,
                                serde_json::to_string(&multi_audience_wrong_azp_id_token)
                                    .unwrap_or_default()
                            ),
                        )
                    } else if request.starts_with("POST /token ")
                        && request.contains("code=multi-audience-valid-azp-code")
                    {
                        (
                            "200 OK",
                            format!(
                                r#"{{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile","id_token":{}}}"#,
                                serde_json::to_string(&multi_audience_valid_azp_id_token)
                                    .unwrap_or_default()
                            ),
                        )
                    } else if request.starts_with("POST /token ") {
                        (
                            "200 OK",
                            r#"{"access_token":"access-token","token_type":"Bearer","scope":"openid email profile"}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /.well-known/openid-configuration ") {
                        let issuer = format!(
                            "http://{}",
                            stream
                                .local_addr()
                                .map(|addr| addr.to_string())
                                .unwrap_or_default()
                        );
                        let body = format!(
                            r#"{{
                                "issuer":"{issuer}",
                                "authorization_endpoint":"{issuer}/authorize",
                                "token_endpoint":"{issuer}/token",
                                "jwks_uri":"{issuer}/keys",
                                "userinfo_endpoint":"{issuer}/userinfo",
                                "revocation_endpoint":"{issuer}/revoke",
                                "end_session_endpoint":"{issuer}/endsession",
                                "introspection_endpoint":"{issuer}/introspection",
                                "token_endpoint_auth_methods_supported":["client_secret_basic","client_secret_post"],
                                "scopes_supported":["openid","email","profile"]
                            }}"#
                        );
                        ("200 OK", body)
                    } else if request.starts_with("GET /mapped-userinfo ") {
                        (
                            "200 OK",
                            r#"{"external_id":"mapped_subject","mail":"mapped-user@example.com","verified":true,"display":"Mapped User","avatar":"https://example.com/mapped.png","department":"Engineering","employee_number":"E-123"}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /mixed-case-userinfo ") {
                        (
                            "200 OK",
                            r#"{"sub":"subject_123","email":"SSO-User@Example.Com","email_verified":true,"name":"SSO User","picture":"https://example.com/avatar.png"}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /missing-sub-userinfo ") {
                        (
                            "200 OK",
                            r#"{"email":"missing-sub@example.com","email_verified":true,"name":"Missing Sub","picture":"https://example.com/avatar.png"}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /fixtures/google/userinfo ") {
                        (
                            "200 OK",
                            r#"{"sub":"google-sub-123","email":"Google.User@Example.COM","email_verified":true,"name":"Google Workspace User","picture":"https://lh3.googleusercontent.com/a/example","hd":"example.com","locale":"en-US"}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /fixtures/google/unverified-userinfo ") {
                        (
                            "200 OK",
                            r#"{"sub":"google-unverified-sub","email":"sso-user@example.com","email_verified":false,"name":"Unverified Google User","picture":"https://lh3.googleusercontent.com/a/unverified","hd":"example.com","locale":"en-US"}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /fixtures/azure/userinfo ") {
                        (
                            "200 OK",
                            r#"{"sub":"azure-sub-456","oid":"azure-oid-456","tid":"tenant-123","preferred_username":"Ada@Contoso.COM","upn":"ada@contoso.com","email_verified":true,"name":"Ada Lovelace"}"#.to_owned(),
                        )
                    } else if request
                        .starts_with("GET /fixtures/azure/missing-preferred-username-userinfo ")
                    {
                        (
                            "200 OK",
                            r#"{"sub":"azure-sub-missing-email","oid":"azure-oid-missing-email","tid":"tenant-123","upn":"ada@contoso.com","email_verified":true,"name":"Ada Lovelace"}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /fixtures/okta/userinfo ") {
                        (
                            "200 OK",
                            r#"{"sub":"okta-sub-789","email":"Okta.User@Example.COM","email_verified":true,"name":"Okta User","zoneinfo":"America/Monterrey","groups":["Engineering","Admins"]}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /fixtures/okta/missing-sub-userinfo ") {
                        (
                            "200 OK",
                            r#"{"email":"Okta.User@Example.COM","email_verified":true,"name":"Okta User","zoneinfo":"America/Monterrey","groups":["Engineering","Admins"]}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /userinfo ") {
                        (
                            "200 OK",
                            r#"{"sub":"subject_123","email":"sso-user@example.com","email_verified":true,"name":"SSO User","picture":"https://example.com/avatar.png"}"#.to_owned(),
                        )
                    } else if request.starts_with("GET /keys ") {
                        ("200 OK", jwks_body)
                    } else {
                        ("404 Not Found", r#"{"error":"not_found"}"#.to_owned())
                    };
                    let response = format!(
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ =
                        tokio::io::AsyncWriteExt::write_all(&mut stream, response.as_bytes()).await;
                });
            }
        });
        Ok(Self {
            base_url,
            token_requests,
        })
    }

    pub(crate) fn token_requests(&self) -> Vec<String> {
        self.token_requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }
}

pub(crate) fn signed_oidc_id_token(
    audience: &str,
    issuer: &str,
) -> Result<(String, Jwk), Box<dyn std::error::Error>> {
    signed_oidc_id_token_with_options(audience, issuer, IdTokenOptions::default())
}

#[derive(Debug, Clone)]
pub(crate) struct IdTokenOptions {
    pub(crate) include_sub: bool,
    pub(crate) include_exp: bool,
    pub(crate) issuer: Option<String>,
    pub(crate) subject: String,
    pub(crate) email: Option<String>,
    pub(crate) email_verified: Option<bool>,
    pub(crate) name: Option<String>,
    pub(crate) picture: Option<String>,
    pub(crate) key_id: String,
    pub(crate) audience_claim: Option<serde_json::Value>,
    pub(crate) extra_claims: Vec<(String, serde_json::Value)>,
}

impl Default for IdTokenOptions {
    fn default() -> Self {
        Self {
            include_sub: true,
            include_exp: true,
            issuer: None,
            subject: "subject_123".to_owned(),
            email: Some("sso-user@example.com".to_owned()),
            email_verified: Some(true),
            name: None,
            picture: None,
            key_id: "sso-test-key".to_owned(),
            audience_claim: None,
            extra_claims: Vec::new(),
        }
    }
}

impl IdTokenOptions {
    fn azure() -> Self {
        Self {
            issuer: Some(
                "https://login.microsoftonline.com/11111111-1111-1111-1111-111111111111/v2.0"
                    .to_owned(),
            ),
            subject: "azure-token-sub-456".to_owned(),
            email: Some("token.user@contoso.com".to_owned()),
            email_verified: Some(true),
            name: Some("Token User".to_owned()),
            key_id: "azure-test-key".to_owned(),
            extra_claims: vec![
                ("oid".to_owned(), json!("azure-token-oid-456")),
                ("tid".to_owned(), json!("tenant-123")),
                (
                    "preferred_username".to_owned(),
                    json!("Token.User@Contoso.COM"),
                ),
            ],
            ..Self::default()
        }
    }
}

pub(crate) fn signed_oidc_id_token_with_options(
    audience: &str,
    issuer: &str,
    options: IdTokenOptions,
) -> Result<(String, Jwk), Box<dyn std::error::Error>> {
    let kid = options.key_id.clone();
    let mut jwk = Jwk::generate_rsa_key(2048)?;
    jwk.set_key_id(&kid);
    jwk.set_algorithm("RS256");
    jwk.set_key_use("sig");

    let signer = Rs256.signer_from_jwk(&jwk)?;
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let mut payload = JwtPayload::new();
    payload.set_claim(
        "aud",
        Some(options.audience_claim.unwrap_or_else(|| json!(audience))),
    )?;
    payload.set_claim(
        "iss",
        Some(json!(options.issuer.as_deref().unwrap_or(issuer))),
    )?;
    if options.include_sub {
        payload.set_claim("sub", Some(json!(options.subject)))?;
    }
    if let Some(email) = options.email {
        payload.set_claim("email", Some(json!(email)))?;
    }
    if let Some(email_verified) = options.email_verified {
        payload.set_claim("email_verified", Some(json!(email_verified)))?;
    }
    if let Some(name) = options.name {
        payload.set_claim("name", Some(json!(name)))?;
    }
    if let Some(picture) = options.picture {
        payload.set_claim("picture", Some(json!(picture)))?;
    }
    for (key, value) in options.extra_claims {
        payload.set_claim(&key, Some(value))?;
    }
    payload.set_claim("iat", Some(json!(now)))?;
    if options.include_exp {
        payload.set_claim("exp", Some(json!(now + 3600)))?;
    }

    let mut header = JwsHeader::new();
    header.set_algorithm("RS256");
    header.set_key_id(&kid);
    let token = jwt::encode_with_signer(&payload, &header, &signer)?;
    let mut public_jwk = jwk.to_public_key()?;
    public_jwk.set_key_id(kid);
    public_jwk.set_algorithm("RS256");
    public_jwk.set_key_use("sig");
    Ok((token, public_jwk))
}
