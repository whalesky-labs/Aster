use pbkdf2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use pbkdf2::Pbkdf2;

use crate::error::{AppError, AppResult};

pub fn hash(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Pbkdf2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| AppError::Validation(format!("密码哈希失败：{error}")))
}

pub fn verify(password: &str, encoded_hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(encoded_hash) else {
        return false;
    };
    Pbkdf2.verify_password(password.as_bytes(), &parsed).is_ok()
}
