use aes_gcm::aead::{Aead, KeyInit, OsRng, rand_core::RngCore};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hkdf::Hkdf;
use sha2::Sha256;

pub const ENCRYPTED_VALUE_PREFIX: &str = "enc:v1:";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

pub struct DerivedKeys {
    header_key: [u8; KEY_LEN],
}

pub fn encrypt_header_value(value: &str, secret_env: &str) -> Result<String, String> {
    let keys = load_derived_keys(secret_env)?;
    encrypt_secret_value(value, &keys.header_key)
        .map_err(|error| format!("failed to encrypt header: {error}"))
}

pub fn decrypt_header_value(value: &str, secret_env: &str) -> Result<String, String> {
    let keys = load_derived_keys(secret_env)?;
    decrypt_secret_value(value, &keys)
}

pub fn is_encrypted_value(value: &str) -> bool {
    value.starts_with(ENCRYPTED_VALUE_PREFIX)
}

fn load_derived_keys(secret_env: &str) -> Result<DerivedKeys, String> {
    let secret = std::env::var(secret_env)
        .map_err(|_| format!("environment variable '{secret_env}' is not set"))?;
    derive_keys(secret.as_bytes()).map_err(|error| error.to_string())
}

fn derive_keys(secret: &[u8]) -> Result<DerivedKeys, hkdf::InvalidLength> {
    let hkdf = Hkdf::<Sha256>::new(None, secret);
    let mut header_key = [0_u8; KEY_LEN];
    hkdf.expand(b"proxy-tools/header", &mut header_key)?;
    Ok(DerivedKeys { header_key })
}

fn encrypt_secret_value(
    value: &str,
    header_key: &[u8; KEY_LEN],
) -> Result<String, aes_gcm::Error> {
    let cipher = Aes256Gcm::new_from_slice(header_key).expect("valid key size");
    let mut nonce_bytes = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce_bytes), value.as_bytes())?;

    Ok(format!(
        "{ENCRYPTED_VALUE_PREFIX}{}:{}",
        STANDARD.encode(nonce_bytes),
        STANDARD.encode(ciphertext)
    ))
}

fn decrypt_secret_value(value: &str, keys: &DerivedKeys) -> Result<String, String> {
    let encoded = value
        .strip_prefix(ENCRYPTED_VALUE_PREFIX)
        .ok_or_else(|| "invalid encrypted header prefix".to_string())?;
    let (nonce_b64, ciphertext_b64) = encoded
        .split_once(':')
        .ok_or_else(|| "invalid encrypted header format".to_string())?;
    let nonce = STANDARD
        .decode(nonce_b64)
        .map_err(|_| "invalid encrypted header nonce".to_string())?;
    let ciphertext = STANDARD
        .decode(ciphertext_b64)
        .map_err(|_| "invalid encrypted header payload".to_string())?;

    if nonce.len() != NONCE_LEN {
        return Err("invalid encrypted header nonce length".to_string());
    }

    let cipher = Aes256Gcm::new_from_slice(&keys.header_key).expect("valid key size");
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| "failed to decrypt encrypted header".to_string())?;

    String::from_utf8(plaintext).map_err(|_| "decrypted header is not valid UTF-8".to_string())
}

#[cfg(test)]
mod tests {
    use super::{decrypt_header_value, encrypt_header_value, is_encrypted_value};

    #[test]
    fn encrypts_and_decrypts_header_value() {
        unsafe {
            std::env::set_var("PROXY_TOOLS_TEST_SECRET", "test-secret");
        }

        let encrypted = encrypt_header_value("Bearer token", "PROXY_TOOLS_TEST_SECRET")
            .expect("value should encrypt");
        assert!(is_encrypted_value(&encrypted));

        let decrypted = decrypt_header_value(&encrypted, "PROXY_TOOLS_TEST_SECRET")
            .expect("value should decrypt");
        assert_eq!(decrypted, "Bearer token");
    }
}
