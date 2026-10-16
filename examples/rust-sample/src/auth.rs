//! Аутентификация. Намеренно нарушает слои: auth -> db напрямую.

use crate::db;

pub fn validate(username: &str) -> bool {
    username.len() >= 3
}

pub fn login(username: &str) -> String {
    if !validate(username) {
        return String::new();
    }
    let user = db::find_user(username);
    format!("token-{}", user)
}

pub fn logout(username: &str) -> bool {
    validate(username)
}

pub fn hash_password(password: &str) -> String {
    format!("hashed:{}", password)
}