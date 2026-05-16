#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FredRateLimitOptions {
    pub key_prefix: String,
}

impl Default for FredRateLimitOptions {
    fn default() -> Self {
        Self {
            key_prefix: "openauth:".to_owned(),
        }
    }
}
