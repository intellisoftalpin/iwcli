//! TOTP (2FA) code generation for `2FAC` fields.

use std::time::{SystemTime, UNIX_EPOCH};

use base32::Alphabet;
use totp_rs::{Algorithm, Builder, Totp};
use zeroize::Zeroizing;

/// Step length used for raw base32 secrets.
const STEP_SECONDS: u64 = 30;

/// Generate the current TOTP code from a secret. Returns (code, seconds_remaining).
pub fn generate_totp(secret: &str) -> Option<(String, u64)> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    generate_totp_at(secret, now)
}

/// Generate the TOTP code for the Unix time `now`. Returns (code, seconds_remaining).
///
/// `secret` is either an `otpauth://` URL or a raw base32 secret (spaces and
/// lower case allowed). Secrets shorter than 128 bits are accepted in both
/// forms: many services still hand out 80-bit secrets.
pub fn generate_totp_at(secret: &str, now: u64) -> Option<(String, u64)> {
    let totp = if secret.starts_with("otpauth://") {
        // Unchecked like raw secrets below: otpauth URLs often carry 80-bit secrets.
        let totp = Totp::from_url_unchecked(secret).ok()?;
        // Values the generator cannot handle (it would divide by zero or overflow).
        if totp.step() == 0 || !(6..=8).contains(&totp.digits()) {
            return None;
        }
        totp
    } else {
        let secret_clean = Zeroizing::new(secret.replace(' ', "").to_uppercase());
        let decoded = base32::decode(Alphabet::Rfc4648 { padding: false }, &secret_clean)?;
        if decoded.is_empty() {
            return None;
        }
        // build_noncompliant: allow secrets shorter than 128 bits.
        Builder::new()
            .with_algorithm(Algorithm::SHA1)
            .with_digits(6)
            .with_skew(1)
            .with_step_duration(STEP_SECONDS)
            .with_secret(decoded)
            .build_noncompliant()
    };

    let step = totp.step();
    let code = totp.generate(now).to_string();
    let seconds_remaining = step - (now % step);

    Some((code, seconds_remaining))
}
