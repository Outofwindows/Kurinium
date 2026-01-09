pub fn decrypt_token(encrypted: &[u8], key: &[u8]) -> String {
    if key.is_empty() { return String::new(); }
    let decrypted: Vec<u8> = encrypted
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()])
        .collect();

    String::from_utf8_lossy(&decrypted).to_string()
}

#[macro_export]
macro_rules! decrypt_token {
    ($encrypted:expr, $key:expr) => {
        $crate::utils::token::decrypt_token($encrypted, $key)
    };
}
