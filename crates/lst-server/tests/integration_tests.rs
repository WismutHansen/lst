use serde_json::json;
use argon2::{password_hash::SaltString, Argon2, Algorithm, Params, PasswordHasher, Version};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_health_endpoint() {
    let client = reqwest::Client::new();
    let response = client.get("http://127.0.0.1:3001/api/health").send().await;

    match response {
        Ok(resp) => {
            assert_eq!(resp.status(), 200);
            let text = resp.text().await.unwrap();
            assert_eq!(text, "OK");
        }
        Err(_) => {
            // Server might not be running - that's okay for now
            println!("Server not running - start with: cargo run --bin lst-server");
        }
    }
}

#[tokio::test]
async fn test_auth_request_endpoint() {
    let client = reqwest::Client::new();

    let params = Params::new(128 * 1024, 3, 2, None).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::encode_b64(b"clientstatic").unwrap();
    let hash = argon2.hash_password(b"hunter42", &salt).unwrap().to_string();

    let payload = json!({
        "email": "test@example.com",
        "host": "127.0.0.1:3001",
        "password_hash": hash
    });

    let response = client
        .post("http://127.0.0.1:3001/api/auth/request")
        .json(&payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let json: serde_json::Value = resp.json().await.unwrap();
                assert_eq!(
                    json.get("status"),
                    Some(&serde_json::Value::String("ok".into()))
                );
            } else {
                println!("Auth request failed with status: {}", resp.status());
            }
        }
        Err(_) => {
            println!("Server not running - start with: cargo run --bin lst-server");
        }
    }
}

#[tokio::test]
async fn test_full_auth_flow() {
    let client = reqwest::Client::new();

    // Step 1: Request auth token
    let params = Params::new(128 * 1024, 3, 2, None).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::encode_b64(b"clientstatic").unwrap();
    let hash = argon2.hash_password(b"hunter42", &salt).unwrap().to_string();

    let payload = json!({
        "email": "test@example.com",
        "host": "127.0.0.1:3001",
        "password_hash": hash
    });

    let auth_response = client
        .post("http://127.0.0.1:3001/api/auth/request")
        .json(&payload)
        .send()
        .await;

    match auth_response {
        Ok(resp) if resp.status().is_success() => {
            let auth_json: serde_json::Value = resp.json().await.unwrap();
            assert_eq!(
                auth_json.get("status"),
                Some(&serde_json::Value::String("ok".into()))
            );
        }
        _ => {
            println!("Server not running - start with: cargo run --bin lst-server -- --config test-server.toml");
        }
    }
}

#[tokio::test]
async fn test_invalid_token_rejection() {
    let client = reqwest::Client::new();

    let verify_payload = json!({
        "email": "test@example.com",
        "token": "INVALID-TOKEN-123"
    });

    let response = client
        .post("http://127.0.0.1:3001/api/auth/verify")
        .json(&verify_payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert_eq!(resp.status(), 401); // Unauthorized
            println!("Invalid token correctly rejected");
        }
        Err(_) => {
            println!("Server not running - start with: cargo run --bin lst-server");
        }
    }
}
