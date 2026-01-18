use argon2::{Argon2, Algorithm, Version, Params};

pub const SALT_SIZE: usize = 32;
pub const NONCE_SIZE: usize = 12;
pub const EXTENSION: &str = ".krn"; // krn = kurinium

pub fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    
    let params = Params::new(19456, 2, 1, Some(32))
        .expect("Invalid Argon2 params");
    
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    
    argon2.hash_password_into(password.as_bytes(), salt, &mut key)
        .expect("Argon2 hashing failed");
    
    key
}
