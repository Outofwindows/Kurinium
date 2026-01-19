use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};

pub fn decrypt_token(encrypted: &[u8], key: &[u8]) -> String {
    if key.is_empty() || key.len() != 32 {
        return String::new();
    }

    let cipher = match Aes256Gcm::new_from_slice(key) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(&key[..12]);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let decrypted = match cipher.decrypt(nonce, encrypted) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };

    String::from_utf8_lossy(&decrypted).to_string()
}

#[macro_export]
macro_rules! decrypt_token {
    ($encrypted:expr, $key:expr) => {
        $crate::utils::token::decrypt_token($encrypted, $key)
    };
}
