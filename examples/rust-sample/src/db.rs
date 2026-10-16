//Доступ к базе данных.

pub fn find_user(username: &str) -> String {
    if username.is_empty() {
        return String::new();
    }
    format!("{}-record", username)
}

pub fn query(table: &str) -> Vec<String> {
    vec![table.to_string()]
}