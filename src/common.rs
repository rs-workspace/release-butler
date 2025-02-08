use hmac::{Hmac, Mac};
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;

pub fn generate_hmac_sha256_hex(body: &[u8], key: &[u8]) -> Option<String> {
    let mut hasher = HmacSha256::new_from_slice(key).expect("Failed to create Hasher");
    hasher.update(body);

    let mut enc_buf = [0u8; 256];
    let Ok(hex) = base16ct::lower::encode_str(&hasher.finalize().into_bytes(), &mut enc_buf) else {
        return None;
    };
    Some(hex.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex() {
        let key = "abc123".as_bytes();
        let body = "Sample Payload".as_bytes();

        let expected =
            String::from("4a91576675ad4b18544e6108b9eaf06c4b5f799cf6ca9bde7ea83c04ec6eff7f"); // Generated from https://www.devglan.com/online-tools/hmac-sha256-online
        let actual = generate_hmac_sha256_hex(body, key).unwrap_or_default();

        assert_eq!(expected, actual)
    }

    #[cfg(feature = "tests")]
    #[test]
    fn test_json_signature() {
        let expected =
            String::from("4ed99f2f66b2328f8af4f8b56874e818033949dc87734b8ac5480c62829fa11a"); // Generated form https://www.devglan.com/online-tools/hmac-sha256-online
        let actual = generate_hmac_sha256_hex(
            crate::tests_utils::payload_template::GITHUB_PUSH,
            crate::tests_utils::DEFAULT_HMAC_KEY.as_bytes(),
        )
        .unwrap_or_default();

        assert_eq!(expected, actual);

        let expected_header_like = format!("sha256={}", expected);
        assert_eq!(
            expected_header_like,
            *crate::tests_utils::payload_template::GITHUB_PUSH_HEX
        )
    }
}
