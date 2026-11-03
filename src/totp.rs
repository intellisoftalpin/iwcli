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

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 appendix B secret ("12345678901234567890") in base32.
    const RFC_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

    #[test]
    fn rfc6238_sha1_vectors_truncated_to_six_digits() {
        // The RFC lists 8-digit codes; the last 6 digits are the 6-digit code.
        for (time, code) in [
            (59, "287082"),
            (1_111_111_109, "081804"),
            (1_111_111_111, "050471"),
            (1_234_567_890, "005924"),
            (2_000_000_000, "279037"),
        ] {
            assert_eq!(
                generate_totp_at(RFC_SECRET, time).unwrap().0,
                code,
                "time {time}"
            );
        }
    }

    #[test]
    fn seconds_remaining_counts_down_to_the_next_step() {
        assert_eq!(generate_totp_at(RFC_SECRET, 59).unwrap().1, 1);
        assert_eq!(generate_totp_at(RFC_SECRET, 60).unwrap().1, 30);
        assert_eq!(generate_totp_at(RFC_SECRET, 75).unwrap().1, 15);
    }

    #[test]
    fn short_80_bit_raw_secret_is_accepted() {
        // 10-byte secret; codes cross-checked with Python hmac (RFC 4226 truncation).
        assert_eq!(
            generate_totp_at("JBSWY3DPEHPK3PXP", 59).unwrap().0,
            "996554"
        );
        assert_eq!(
            generate_totp_at("JBSWY3DPEHPK3PXP", 1_700_000_000)
                .unwrap()
                .0,
            "324550"
        );
    }

    #[test]
    fn raw_secret_ignores_spaces_and_case() {
        let plain = generate_totp_at("JBSWY3DPEHPK3PXP", 1_700_000_000).unwrap();
        let spaced = generate_totp_at("jbsw y3dp ehpk 3pxp", 1_700_000_000).unwrap();
        assert_eq!(plain, spaced);
    }

    #[test]
    fn otpauth_url_gives_the_same_code_as_the_raw_secret() {
        let url = format!("otpauth://totp/Example:alice?secret={RFC_SECRET}&issuer=Example");
        assert_eq!(
            generate_totp_at(&url, 1_234_567_890),
            generate_totp_at(RFC_SECRET, 1_234_567_890)
        );
    }

    #[test]
    fn otpauth_url_with_short_secret_is_accepted() {
        let url = "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example";
        assert_eq!(generate_totp_at(url, 59).unwrap().0, "996554");
    }

    #[test]
    fn otpauth_url_period_sets_seconds_remaining() {
        let url = "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&period=60";
        let (code, remaining) = generate_totp_at(url, 1_700_000_000).unwrap();
        assert_eq!(code, "508648");
        assert_eq!(remaining, 60 - (1_700_000_000 % 60));
    }

    #[test]
    fn otpauth_url_with_zero_period_is_refused() {
        let url = "otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&period=0";
        assert_eq!(generate_totp_at(url, 59), None);
    }

    #[test]
    fn invalid_secrets_give_no_code() {
        assert_eq!(generate_totp_at("", 59), None);
        assert_eq!(generate_totp_at("not base32!", 59), None);
        assert_eq!(
            generate_totp_at("otpauth://totp/x?issuer=nosecret", 59),
            None
        );
        assert_eq!(generate_totp_at("otpauth://not a url", 59), None);
    }

    #[test]
    fn current_code_has_six_digits() {
        let (code, remaining) = generate_totp(RFC_SECRET).unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
        assert!((1..=30).contains(&remaining));
    }
}
