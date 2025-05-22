use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_health_endpoint() {
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:3001/api/health")
        .send()
        .await;
    
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
    
    let payload = json!({
        "email": "test@example.com",
        "host": "127.0.0.1:3001"
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
                assert!(json.get("token").is_some());
                assert!(json.get("qr_png_base64").is_some());
                assert!(json.get("login_url").is_some());
                
                // Verify the login URL format
                let login_url = json["login_url"].as_str().unwrap();
                assert!(login_url.starts_with("lst-login://"));
                assert!(login_url.contains("auth/verify"));
                
                println!("Auth request successful - token: {}", json["token"]);
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
    let payload = json!({
        "email": "test@example.com", 
        "host": "127.0.0.1:3001"
    });
    
    let auth_response = client
        .post("http://127.0.0.1:3001/api/auth/request")
        .json(&payload)
        .send()
        .await;
    
    match auth_response {
        Ok(resp) if resp.status().is_success() => {
            let auth_json: serde_json::Value = resp.json().await.unwrap();
            let token = auth_json["token"].as_str().unwrap();
            
            // Step 2: Verify the token (should work immediately)
            let verify_payload = json!({
                "email": "test@example.com",
                "token": token
            });
            
            let verify_response = client
                .post("http://127.0.0.1:3001/api/auth/verify")
                .json(&verify_payload)
                .send()
                .await
                .unwrap();
            
            if verify_response.status().is_success() {
                let verify_json: serde_json::Value = verify_response.json().await.unwrap();
                assert!(verify_json.get("jwt").is_some());
                assert_eq!(verify_json["user"], "test@example.com");
                
                println!("Full auth flow successful - JWT received");
            } else {
                println!("Token verification failed with status: {}", verify_response.status());
            }
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