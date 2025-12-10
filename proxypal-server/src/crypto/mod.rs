use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;

const NONCE_SIZE: usize = 12;

fn get_encryption_key() -> Result<[u8; 32]> {
    let key_str = std::env::var("ENCRYPTION_KEY")
        .context("ENCRYPTION_KEY environment variable not set")?;

    if let Ok(bytes) = hex::decode(&key_str) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }

    if let Ok(bytes) = BASE64.decode(&key_str) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }

    Err(anyhow!(
        "ENCRYPTION_KEY must be a 32-byte key encoded as hex (64 chars) or base64 (44 chars)"
    ))
}

pub fn encrypt_tokens(tokens: &serde_json::Value) -> Result<String> {
    let key_bytes = get_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

    let plaintext = serde_json::to_vec(tokens)?;

    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(combined))
}

pub fn decrypt_tokens(encrypted: &str) -> Result<serde_json::Value> {
    let key_bytes = get_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;

    let combined = BASE64.decode(encrypted).context("Invalid base64")?;

    if combined.len() < NONCE_SIZE {
        return Err(anyhow!("Encrypted data too short"));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow!("Decryption failed: invalid key or corrupted data"))?;

    serde_json::from_slice(&plaintext).context("Failed to parse decrypted JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;

    fn with_test_key<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        std::env::set_var("ENCRYPTION_KEY", key);
        let result = f();
        std::env::remove_var("ENCRYPTION_KEY");
        result
    }

    #[test]
    #[serial]
    fn encrypt_and_decrypt_roundtrip() {
        with_test_key(|| {
            let tokens = json!({
                "access_token": "sk-abc123",
                "refresh_token": "rt-xyz789",
                "expires_at": 1234567890
            });

            let encrypted = encrypt_tokens(&tokens).expect("encryption should succeed");
            let decrypted = decrypt_tokens(&encrypted).expect("decryption should succeed");

            assert_eq!(tokens, decrypted);
        });
    }

    #[test]
    #[serial]
    fn encryption_uses_random_nonce() {
        with_test_key(|| {
            let tokens = json!({"token": "same-value"});

            let encrypted1 = encrypt_tokens(&tokens).expect("first encryption");
            let encrypted2 = encrypt_tokens(&tokens).expect("second encryption");

            assert_ne!(
                encrypted1, encrypted2,
                "Same plaintext should produce different ciphertext due to random nonce"
            );
        });
    }

    #[test]
    #[serial]
    fn decrypt_with_wrong_key_fails() {
        let encrypted = with_test_key(|| {
            let tokens = json!({"secret": "data"});
            encrypt_tokens(&tokens).expect("encryption should succeed")
        });

        let wrong_key = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        std::env::set_var("ENCRYPTION_KEY", wrong_key);
        let result = decrypt_tokens(&encrypted);
        std::env::remove_var("ENCRYPTION_KEY");

        assert!(result.is_err(), "Decryption with wrong key should fail");
    }

    #[test]
    #[serial]
    fn supports_base64_key() {
        let key_base64 = BASE64.encode([0x42u8; 32]);
        std::env::set_var("ENCRYPTION_KEY", &key_base64);

        let tokens = json!({"test": "value"});
        let encrypted = encrypt_tokens(&tokens).expect("encryption with base64 key");
        let decrypted = decrypt_tokens(&encrypted).expect("decryption with base64 key");

        std::env::remove_var("ENCRYPTION_KEY");
        assert_eq!(tokens, decrypted);
    }
}
