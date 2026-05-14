use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateUserInput {
    pub id: Option<String>,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    pub image: Option<String>,
    pub username: Option<String>,
    pub display_username: Option<String>,
}

impl CreateUserInput {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            email: email.into(),
            email_verified: false,
            image: None,
            username: None,
            display_username: None,
        }
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    #[must_use]
    pub fn email_verified(mut self, email_verified: bool) -> Self {
        self.email_verified = email_verified;
        self
    }

    #[must_use]
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = Some(image.into());
        self
    }

    #[must_use]
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    #[must_use]
    pub fn display_username(mut self, display_username: impl Into<String>) -> Self {
        self.display_username = Some(display_username.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCredentialAccountInput {
    pub id: Option<String>,
    pub user_id: String,
    pub password_hash: String,
}

impl CreateCredentialAccountInput {
    pub fn new(user_id: impl Into<String>, password_hash: impl Into<String>) -> Self {
        Self {
            id: None,
            user_id: user_id.into(),
            password_hash: password_hash.into(),
        }
    }

    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateOAuthAccountInput {
    pub id: Option<String>,
    pub provider_id: String,
    pub account_id: String,
    pub user_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub access_token_expires_at: Option<OffsetDateTime>,
    pub refresh_token_expires_at: Option<OffsetDateTime>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateAccountInput {
    pub access_token: Option<Option<String>>,
    pub refresh_token: Option<Option<String>>,
    pub id_token: Option<Option<String>>,
    pub access_token_expires_at: Option<Option<OffsetDateTime>>,
    pub refresh_token_expires_at: Option<Option<OffsetDateTime>>,
    pub scope: Option<Option<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateUserInput {
    pub name: Option<String>,
    pub image: Option<Option<String>>,
    pub username: Option<Option<String>>,
    pub display_username: Option<Option<String>>,
}

impl UpdateUserInput {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[must_use]
    pub fn image(mut self, image: Option<String>) -> Self {
        self.image = Some(image);
        self
    }

    #[must_use]
    pub fn username(mut self, username: Option<String>) -> Self {
        self.username = Some(username);
        self
    }

    #[must_use]
    pub fn display_username(mut self, display_username: Option<String>) -> Self {
        self.display_username = Some(display_username);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.image.is_none()
            && self.username.is_none()
            && self.display_username.is_none()
    }
}
