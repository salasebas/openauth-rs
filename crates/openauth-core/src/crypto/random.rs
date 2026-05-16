//! Secure random string generation.

use rand::rngs::OsRng;
use rand::RngCore;

const OPENAUTH_CHARSET: &[u8; 64] =
    b"abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-_";

/// Generate a cryptographically random string using OpenAuth's URL-safe charset.
pub fn generate_random_string(length: usize) -> String {
    let mut output = String::with_capacity(length);
    let mut random = vec![0_u8; length];
    OsRng.fill_bytes(&mut random);

    for byte in random {
        let index = usize::from(byte & 0b0011_1111);
        output.push(char::from(OPENAUTH_CHARSET[index]));
    }

    output
}
