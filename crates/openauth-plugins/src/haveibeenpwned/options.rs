//! Options for the Have I Been Pwned plugin.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaveIBeenPwnedOptions {
    pub custom_password_compromised_message: Option<String>,
    pub paths: Vec<String>,
    pub enabled: bool,
}

impl Default for HaveIBeenPwnedOptions {
    fn default() -> Self {
        Self {
            custom_password_compromised_message: None,
            paths: vec![
                "/sign-up/email".to_owned(),
                "/change-password".to_owned(),
                "/reset-password".to_owned(),
            ],
            enabled: true,
        }
    }
}
