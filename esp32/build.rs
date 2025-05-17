use std::fs;

fn main() {
    embuild::espidf::sysenv::output();

    // Olvasd be a .env fájlt
    let contents = fs::read_to_string(".env").expect("Failed to read .env file");

    for line in contents.lines() {
        if let Some((key, value)) = line.split_once('=') {
            println!("cargo:rustc-env={}={}", key.trim(), value.trim());
        }
    }
}
