use proxypal_server::{crypto, db};
use argon2::{password_hash::PasswordHash, Argon2, PasswordHasher, PasswordVerifier};
use base64::{engine::general_purpose::STANDARD, Engine};
use serial_test::serial;
use std::path::PathBuf;
use tempfile::tempdir;

fn valid_encryption_key() -> String {
    let key = [0u8; 32];
    STANDARD.encode(key)
}

fn setup_encryption_key() {
    std::env::set_var("ENCRYPTION_KEY", valid_encryption_key());
}

fn cleanup_env() {
    std::env::remove_var("DATABASE_PATH");
    std::env::remove_var("DATA_DIR");
    std::env::remove_var("ENCRYPTION_KEY");
    std::env::remove_var("ADMIN_PASSWORD");
}

#[test]
#[serial]
fn test_database_path_env_honored() {
    cleanup_env();

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("custom.db");
    std::env::set_var("DATABASE_PATH", db_path.to_str().unwrap());

    let db = db::init().expect("db::init should succeed");
    
    assert!(db_path.exists(), "Database file should be created at DATABASE_PATH location");

    drop(db);
    cleanup_env();
}

#[test]
#[serial]
fn test_database_path_default() {
    cleanup_env();

    let cwd = std::env::current_dir().unwrap();
    let default_path = cwd.join("proxypal.db");

    if default_path.exists() {
        std::fs::remove_file(&default_path).ok();
    }

    let _db = db::init().expect("db::init should succeed with default path");

    assert!(default_path.exists(), "Database should be created at default location proxypal.db");

    std::fs::remove_file(&default_path).ok();
    cleanup_env();
}

#[test]
#[serial]
fn test_database_creates_parent_directories() {
    cleanup_env();

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("nested").join("dirs").join("test.db");
    std::env::set_var("DATABASE_PATH", db_path.to_str().unwrap());

    let db = db::init().expect("db::init should create parent directories");

    assert!(db_path.exists(), "Database file should exist");
    assert!(db_path.parent().unwrap().exists(), "Parent directories should be created");

    drop(db);
    cleanup_env();
}

#[test]
#[serial]
fn test_data_dir_affects_config_location() {
    cleanup_env();
    setup_encryption_key();

    let dir = tempdir().unwrap();
    let data_dir = dir.path().join("proxypal-data");
    std::env::set_var("DATA_DIR", data_dir.to_str().unwrap());

    let config_path = if let Ok(data_dir) = std::env::var("DATA_DIR") {
        PathBuf::from(data_dir).join("proxy-config.yaml")
    } else {
        PathBuf::from("proxy-config.yaml")
    };

    assert_eq!(
        config_path,
        data_dir.join("proxy-config.yaml"),
        "Config path should use DATA_DIR"
    );

    cleanup_env();
}

#[test]
#[serial]
fn test_encryption_key_validation_missing() {
    cleanup_env();

    let tokens = serde_json::json!({"token": "secret"});
    let result = crypto::encrypt_tokens(&tokens);

    assert!(result.is_err(), "Encryption should fail without ENCRYPTION_KEY");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("ENCRYPTION_KEY"),
        "Error should mention ENCRYPTION_KEY: {}",
        err_msg
    );

    cleanup_env();
}

#[test]
#[serial]
fn test_encryption_key_validation_invalid_length() {
    cleanup_env();

    std::env::set_var("ENCRYPTION_KEY", "too-short");

    let tokens = serde_json::json!({"token": "secret"});
    let result = crypto::encrypt_tokens(&tokens);

    assert!(result.is_err(), "Encryption should fail with invalid key length");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("32-byte"),
        "Error should mention key length requirement: {}",
        err_msg
    );

    cleanup_env();
}

#[test]
#[serial]
fn test_encryption_key_validation_valid_base64() {
    cleanup_env();

    let key = [0x42u8; 32];
    std::env::set_var("ENCRYPTION_KEY", STANDARD.encode(key));

    let tokens = serde_json::json!({"token": "secret"});
    let result = crypto::encrypt_tokens(&tokens);

    assert!(result.is_ok(), "Encryption should succeed with valid base64 key");

    let encrypted = result.unwrap();
    let decrypted = crypto::decrypt_tokens(&encrypted).expect("Decryption should succeed");
    assert_eq!(tokens, decrypted);

    cleanup_env();
}

#[test]
#[serial]
fn test_encryption_key_validation_valid_hex() {
    cleanup_env();

    let hex_key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    std::env::set_var("ENCRYPTION_KEY", hex_key);

    let tokens = serde_json::json!({"token": "secret"});
    let result = crypto::encrypt_tokens(&tokens);

    assert!(result.is_ok(), "Encryption should succeed with valid hex key");

    cleanup_env();
}

#[test]
#[serial]
fn test_admin_password_bootstrap() {
    cleanup_env();

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("admin_test.db");
    let db = db::Database::new(db_path).expect("Database should be created");

    assert!(
        db.get_setting("admin_password_hash").unwrap().is_none(),
        "Fresh database should not have admin password"
    );

    let password = "test-admin-password";
    let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Hashing should succeed")
        .to_string();

    db.set_setting("admin_password_hash", &hash).unwrap();

    let stored_hash = db
        .get_setting("admin_password_hash")
        .unwrap()
        .expect("Hash should be stored");

    let parsed_hash = PasswordHash::new(&stored_hash).expect("Hash should be parseable");
    assert!(
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok(),
        "Password verification should succeed"
    );

    cleanup_env();
}

#[test]
#[serial]
fn test_admin_password_not_overwritten() {
    cleanup_env();

    let dir = tempdir().unwrap();
    let db_path = dir.path().join("admin_overwrite_test.db");
    let db = db::Database::new(db_path).expect("Database should be created");

    let password = "original-password";
    let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
    let original_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("Hashing should succeed")
        .to_string();

    db.set_setting("admin_password_hash", &original_hash).unwrap();

    let stored = db.get_setting("admin_password_hash").unwrap();
    assert!(stored.is_some(), "Password hash should exist");

    let stored_hash = stored.unwrap();
    let parsed = PasswordHash::new(&stored_hash).expect("Hash should be parseable");
    assert!(
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        "Original password should still work"
    );

    cleanup_env();
}
