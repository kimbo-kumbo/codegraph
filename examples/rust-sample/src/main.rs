//Точка входа примера. Демонстрирует кросс-файловые вызовы в Rust.

mod auth;
mod db;

fn main() {
    let username = "alice";
    let token = auth::login(username);
    if !token.is_empty() {
        println!("token: {}", token);
        auth::logout(username);
    }
}