//! Better Auth secret-rotation envelope helpers.

const ENVELOPE_PREFIX: &str = "$ba$";

/// Parsed Better Auth encrypted payload envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    pub version: u32,
    pub ciphertext: String,
}

/// Parse `$ba$<version>$<ciphertext>` payloads.
pub fn parse_envelope(data: &str) -> Option<Envelope> {
    let rest = data.strip_prefix(ENVELOPE_PREFIX)?;
    let (version, ciphertext) = rest.split_once('$')?;
    if version.starts_with('-') {
        return None;
    }
    let version = version.parse::<u32>().ok()?;

    Some(Envelope {
        version,
        ciphertext: ciphertext.to_owned(),
    })
}

/// Format a ciphertext with Better Auth's secret-rotation envelope.
pub fn format_envelope(version: u32, ciphertext: &str) -> String {
    format!("{ENVELOPE_PREFIX}{version}${ciphertext}")
}
