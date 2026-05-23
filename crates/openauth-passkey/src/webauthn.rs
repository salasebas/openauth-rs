use base64::Engine;
use openauth_core::error::OpenAuthError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, Credential, DiscoverableAuthentication, DiscoverableKey,
    PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse, Webauthn, WebauthnBuilder,
};

use crate::options::{PasskeyRegistrationUser, RegistrationWebAuthnOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebAuthnConfig {
    pub rp_id: String,
    pub rp_name: String,
    pub origins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasskeyRegistrationStart {
    pub options: Value,
    pub state: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasskeyAuthenticationStart {
    pub options: Value,
    pub state: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedPasskeyCredential {
    pub credential_id: String,
    pub public_key: String,
    pub counter: u32,
    pub device_type: String,
    pub backed_up: bool,
    pub transports: Option<String>,
    pub aaguid: Option<String>,
    pub credential: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedAuthentication {
    pub credential: Option<Value>,
    pub new_counter: u32,
}

pub trait PasskeyWebAuthnBackend: Send + Sync {
    fn start_registration(
        &self,
        config: WebAuthnConfig,
        user: &PasskeyRegistrationUser,
        exclude_credentials: Vec<Value>,
        options: RegistrationWebAuthnOptions,
    ) -> Result<PasskeyRegistrationStart, OpenAuthError>;

    fn finish_registration(
        &self,
        config: WebAuthnConfig,
        response: Value,
        state: Value,
    ) -> Result<VerifiedPasskeyCredential, OpenAuthError> {
        let _ = (config, response, state);
        Err(OpenAuthError::Api(
            "passkey registration verification is not implemented".to_owned(),
        ))
    }

    fn start_authentication(
        &self,
        config: WebAuthnConfig,
        credentials: Vec<Value>,
        extensions: Option<Value>,
    ) -> Result<PasskeyAuthenticationStart, OpenAuthError>;

    fn finish_authentication(
        &self,
        config: WebAuthnConfig,
        response: Value,
        state: Value,
        credential: Option<Value>,
    ) -> Result<VerifiedAuthentication, OpenAuthError> {
        let _ = (config, response, state, credential);
        Err(OpenAuthError::Api(
            "passkey authentication verification is not implemented".to_owned(),
        ))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RealPasskeyWebAuthnBackend;

impl PasskeyWebAuthnBackend for RealPasskeyWebAuthnBackend {
    fn start_registration(
        &self,
        config: WebAuthnConfig,
        user: &PasskeyRegistrationUser,
        exclude_credentials: Vec<Value>,
        request_options: RegistrationWebAuthnOptions,
    ) -> Result<PasskeyRegistrationStart, OpenAuthError> {
        let webauthn = webauthn(&config)?;
        let exclude = exclude_credentials
            .into_iter()
            .map(|value| {
                serde_json::from_value::<Credential>(value).map(|credential| credential.cred_id)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| OpenAuthError::Api(error.to_string()))?;
        let user_id = stable_user_uuid(&user.id);
        let display_name = user.display_name.as_deref().unwrap_or(&user.name);
        let (options, state) = webauthn
            .start_passkey_registration(user_id, &user.name, display_name, Some(exclude))
            .map_err(|error| OpenAuthError::Api(error.to_string()))?;
        let mut options = option_value(options)?;
        apply_registration_request_options(&mut options, &request_options);
        Ok(PasskeyRegistrationStart {
            options,
            state: serde_json::to_value(state).map_err(json_error)?,
        })
    }

    fn finish_registration(
        &self,
        config: WebAuthnConfig,
        response: Value,
        state: Value,
    ) -> Result<VerifiedPasskeyCredential, OpenAuthError> {
        let webauthn = webauthn(&config)?;
        let response = serde_json::from_value::<RegisterPublicKeyCredential>(response)
            .map_err(|error| OpenAuthError::Api(error.to_string()))?;
        let state = serde_json::from_value::<PasskeyRegistration>(state).map_err(json_error)?;
        let passkey = webauthn
            .finish_passkey_registration(&response, &state)
            .map_err(|error| OpenAuthError::Api(error.to_string()))?;
        credential_output(passkey)
    }

    fn start_authentication(
        &self,
        config: WebAuthnConfig,
        credentials: Vec<Value>,
        extensions: Option<Value>,
    ) -> Result<PasskeyAuthenticationStart, OpenAuthError> {
        let webauthn = webauthn(&config)?;
        if credentials.is_empty() {
            let (options, state) = webauthn
                .start_discoverable_authentication()
                .map_err(|error| OpenAuthError::Api(error.to_string()))?;
            let mut options = auth_option_value(options)?;
            apply_authentication_request_options(&mut options, extensions);
            return Ok(PasskeyAuthenticationStart {
                options,
                state: serde_json::to_value(StoredAuthenticationState::Discoverable(state))
                    .map_err(json_error)?,
            });
        }
        let passkeys = credentials
            .into_iter()
            .map(credential_value_to_passkey)
            .collect::<Result<Vec<_>, _>>()?;
        let (options, state) = webauthn
            .start_passkey_authentication(&passkeys)
            .map_err(|error| OpenAuthError::Api(error.to_string()))?;
        let mut options = auth_option_value(options)?;
        apply_authentication_request_options(&mut options, extensions);
        Ok(PasskeyAuthenticationStart {
            options,
            state: serde_json::to_value(StoredAuthenticationState::Passkey(state))
                .map_err(json_error)?,
        })
    }

    fn finish_authentication(
        &self,
        config: WebAuthnConfig,
        response: Value,
        state: Value,
        credential: Option<Value>,
    ) -> Result<VerifiedAuthentication, OpenAuthError> {
        let webauthn = webauthn(&config)?;
        let response = serde_json::from_value::<PublicKeyCredential>(response)
            .map_err(|error| OpenAuthError::Api(error.to_string()))?;
        let state =
            serde_json::from_value::<StoredAuthenticationState>(state).map_err(json_error)?;
        let credential = credential.map(credential_value_to_passkey).transpose()?;
        let result = match state {
            StoredAuthenticationState::Passkey(state) => webauthn
                .finish_passkey_authentication(&response, &state)
                .map_err(|error| OpenAuthError::Api(error.to_string()))?,
            StoredAuthenticationState::Discoverable(state) => {
                let Some(credential) = credential.as_ref() else {
                    return Err(OpenAuthError::Api(
                        "passkey credential is required".to_owned(),
                    ));
                };
                let discoverable = DiscoverableKey::from(credential);
                webauthn
                    .finish_discoverable_authentication(&response, state, &[discoverable])
                    .map_err(|error| OpenAuthError::Api(error.to_string()))?
            }
        };
        let updated_credential = credential.and_then(|mut passkey| {
            passkey
                .update_credential(&result)
                .and_then(|changed| changed.then_some(passkey))
        });
        Ok(VerifiedAuthentication {
            credential: updated_credential
                .map(|passkey| serde_json::to_value(passkey).map_err(json_error))
                .transpose()?,
            new_counter: result.counter(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum StoredAuthenticationState {
    Passkey(PasskeyAuthentication),
    Discoverable(DiscoverableAuthentication),
}

fn webauthn(config: &WebAuthnConfig) -> Result<Webauthn, OpenAuthError> {
    let primary_origin = config
        .origins
        .first()
        .ok_or_else(|| OpenAuthError::InvalidConfig("passkey origin is required".to_owned()))?;
    let primary =
        Url::parse(primary_origin).map_err(|error| OpenAuthError::Api(error.to_string()))?;
    let mut builder = WebauthnBuilder::new(&config.rp_id, &primary)
        .map_err(|error| OpenAuthError::Api(error.to_string()))?
        .rp_name(&config.rp_name)
        .allow_any_port(true);
    for origin in config.origins.iter().skip(1) {
        let origin = Url::parse(origin).map_err(|error| OpenAuthError::Api(error.to_string()))?;
        builder = builder.append_allowed_origin(&origin);
    }
    builder
        .build()
        .map_err(|error| OpenAuthError::Api(error.to_string()))
}

fn option_value(options: CreationChallengeResponse) -> Result<Value, OpenAuthError> {
    serde_json::to_value(options)
        .map(|mut value| value.pointer_mut("/publicKey").cloned().unwrap_or(value))
        .map_err(json_error)
}

fn auth_option_value(options: RequestChallengeResponse) -> Result<Value, OpenAuthError> {
    serde_json::to_value(options)
        .map(|mut value| value.pointer_mut("/publicKey").cloned().unwrap_or(value))
        .map_err(json_error)
}

fn apply_registration_request_options(
    options: &mut Value,
    request_options: &RegistrationWebAuthnOptions,
) {
    options["authenticatorSelection"] = request_options.authenticator_selection.to_json();
    if let Some(extensions) = &request_options.extensions {
        options["extensions"] = extensions.clone();
    }
}

fn apply_authentication_request_options(options: &mut Value, extensions: Option<Value>) {
    options["userVerification"] = Value::String("preferred".to_owned());
    if let Some(extensions) = extensions {
        options["extensions"] = extensions;
    }
}

fn credential_value_to_passkey(
    value: Value,
) -> Result<webauthn_rs::prelude::Passkey, OpenAuthError> {
    serde_json::from_value::<webauthn_rs::prelude::Passkey>(value).map_err(json_error)
}

fn credential_output(
    passkey: webauthn_rs::prelude::Passkey,
) -> Result<VerifiedPasskeyCredential, OpenAuthError> {
    let credential = Credential::from(passkey.clone());
    let credential_id = serde_json::to_value(&credential.cred_id)
        .and_then(serde_json::from_value::<String>)
        .unwrap_or_else(|_| format!("{:?}", credential.cred_id));
    let public_key = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_vec(&credential.cred).map_err(json_error)?);
    let transports = credential.transports.as_ref().map(|values| {
        values
            .iter()
            .map(|value| {
                serde_json::to_value(value)
                    .ok()
                    .and_then(|value| serde_json::from_value::<String>(value).ok())
                    .unwrap_or_else(|| format!("{value:?}").to_ascii_lowercase())
            })
            .collect::<Vec<_>>()
            .join(",")
    });
    Ok(VerifiedPasskeyCredential {
        credential_id,
        public_key,
        counter: credential.counter,
        device_type: if credential.backup_eligible {
            "multiDevice".to_owned()
        } else {
            "singleDevice".to_owned()
        },
        backed_up: credential.backup_state,
        transports,
        aaguid: None,
        credential: serde_json::to_value(passkey).map_err(json_error)?,
    })
}

fn stable_user_uuid(user_id: &str) -> Uuid {
    // WebAuthn user handles are intentionally stable per OpenAuth user so
    // authenticators can recognize the same account across credential updates.
    // If we later support passkey-first anonymous enrollment, that flow should
    // store a random per-registration user_handle in credential metadata.
    Uuid::new_v5(&Uuid::NAMESPACE_URL, user_id.as_bytes())
}

fn json_error(error: serde_json::Error) -> OpenAuthError {
    OpenAuthError::Api(error.to_string())
}
