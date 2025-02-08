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
