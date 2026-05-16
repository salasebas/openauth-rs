use std::collections::BTreeMap;

use crate::db::{DbFieldType, DbValue};

/// User lifecycle configuration.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserOptions {
    pub change_email: ChangeEmailOptions,
    pub delete_user: DeleteUserOptions,
    pub additional_fields: BTreeMap<String, UserAdditionalField>,
}

impl UserOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> Self {
        Self::new()
    }

    #[must_use]
    pub fn change_email(mut self, change_email: ChangeEmailOptions) -> Self {
        self.change_email = change_email;
        self
    }

    #[must_use]
    pub fn delete_user(mut self, delete_user: DeleteUserOptions) -> Self {
        self.delete_user = delete_user;
        self
    }

    #[must_use]
    pub fn additional_field(mut self, name: impl Into<String>, field: UserAdditionalField) -> Self {
        self.additional_fields.insert(name.into(), field);
        self
    }
}

/// Runtime metadata for custom user fields accepted by user-writing endpoints.
#[derive(Debug, Clone, PartialEq)]
pub struct UserAdditionalField {
    pub field_type: DbFieldType,
    pub required: bool,
    pub input: bool,
    pub returned: bool,
    pub default_value: Option<DbValue>,
    pub db_name: Option<String>,
}

impl UserAdditionalField {
    pub fn new(field_type: DbFieldType) -> Self {
        Self {
            field_type,
            required: true,
            input: true,
            returned: true,
            default_value: None,
            db_name: None,
        }
    }

    #[must_use]
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    #[must_use]
    pub fn generated(mut self) -> Self {
        self.input = false;
        self
    }

    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.returned = false;
        self
    }

    #[must_use]
    pub fn default_value(mut self, value: DbValue) -> Self {
        self.default_value = Some(value);
        self
    }

    #[must_use]
    pub fn db_name(mut self, db_name: impl Into<String>) -> Self {
        self.db_name = Some(db_name.into());
        self
    }
}

/// Email change behavior.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeEmailOptions {
    pub enabled: bool,
    pub update_email_without_verification: bool,
}

impl ChangeEmailOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> Self {
        Self::new()
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn update_email_without_verification(mut self, enabled: bool) -> Self {
        self.update_email_without_verification = enabled;
        self
    }
}

/// User deletion behavior.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeleteUserOptions {
    pub enabled: bool,
}

impl DeleteUserOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> Self {
        Self::new()
    }

    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}
