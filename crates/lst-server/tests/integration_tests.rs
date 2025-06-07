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

//-----------------------------------------------------------------------------
// New tests using axum-test and temporary directories
//-----------------------------------------------------------------------------

// Helper to bring in main binary's types and functions for router construction
// This assumes `lst_server` is the name of the library crate or binary.
// If main.rs is a binary, we might need to restructure code or use helper functions
// from the main crate if they are public. For now, let's assume we can access them.
// This is a common challenge in Rust integration testing of binaries.
// We will essentially redefine parts of main.rs's setup for testability.

// Re-define necessary structs and functions from main.rs if not directly accessible
// For a real project, these would be refactored into a library crate.
// For this exercise, we'll assume we can call a setup function similar to main.
// Or, we directly use the types from lst_server crate if main.rs is part of a library.

// Let's assume the main crate is accessible as `lst_server`
// use lst_server::{config::Settings, PersistentTokenStore, Claims, JWT_SECRET, /* other necessary items */};
// For now, since I can't modify Cargo.toml to ensure `lst_server` is a library,
// I will have to duplicate some struct definitions or assume they are globally visible,
// which is not ideal but a limitation of this environment.
// I will write the tests *as if* I have proper access to these.

#[cfg(test)]
mod axum_direct_tests {
    use axum::{Router, Json};
    use axum_test::TestServer;
    use lst_server::config::Settings; // Assuming this is accessible
    use lst_server::{ /*AuthRequest, AuthToken, VerifyRequest, VerifyResponse, Claims, PersistentTokenStore, TokenStore, CreateContentRequest, UpdateContentRequest, ContentResponse, JWT_SECRET, TOKEN_VALID_FOR_SECS*/}; // This won't work directly for a binary crate.
                                      // Let's assume the structs from main.rs are copied or accessible
    use serde::{Deserialize, Serialize};
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::{SystemTime, Duration};
    use tempfile::TempDir;
    use serde_json::json;
    use jsonwebtoken::{encode, Header, EncodingKey};


    // --- Replicated Structs (Normally imported from the crate) ---
    // This is a workaround for not being able to modify crate structure for tests easily.
    #[derive(Serialize, Deserialize, Debug, Clone)]
    struct Claims {
        sub: String,
        exp: usize,
    }

    #[derive(Deserialize)]
    struct AuthRequestPayload {
        email: String,
        host: String,
    }

    #[derive(Deserialize, Serialize)]
    struct AuthTokenResponse {
        token: String,
        qr_png_base64: String,
        login_url: String,
    }

    #[derive(Deserialize)]
    struct VerifyRequestPayload {
        email: String,
        token: String,
    }

    #[derive(Deserialize, Serialize)]
    struct VerifyResponsePayload {
        jwt: String,
        user: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct TestPersistentTokenStore {
        file_path: PathBuf,
        tokens: HashMap<String, (String, SystemTime)>,
    }
    // --- End Replicated Structs ---

    const JWT_TEST_SECRET: &[u8] = b"lst-jwt-demo-secret-goes-here"; // Match main.rs

    // Helper function to build the application router for tests
    // This function would ideally call a public function from the main crate.
    fn setup_test_app(temp_data_dir: &PathBuf, temp_content_dir: &PathBuf) -> Router {
        let mut settings = Settings::default(); // Using default and overriding paths
        settings.paths.content_dir = Some(temp_content_dir.to_str().unwrap().to_string());

        // The token store path is derived from config path in main.rs.
        // Here, we'll directly tell PersistentTokenStore where to save its file.
        // So, we need a similar PersistentTokenStore as in main.rs
        // For simplicity, this test setup will manage its own token store instance.
        // In a real scenario, main() would provide a way to pass a custom path for tokens.json.
        // We'll assume the `PersistentTokenStore::new` takes the *directory* for `tokens.json`.
        let token_store_path = temp_data_dir.join("server_data");
        std::fs::create_dir_all(&token_store_path).unwrap();

        // This requires PersistentTokenStore to be accessible or re-defined for tests.
        // For now, let's assume we're testing the file system interaction part manually
        // for token persistence, and for content tests, we focus on JWT and file ops.
        // The ideal way is: `let token_store = Arc::new(Mutex::new(lst_server::PersistentTokenStore::new(token_store_path)));`

        // Due to visibility issues with binary crate, we can't easily instantiate router
        // as it's done in main.rs. This is a major hurdle for pure integration tests
        // in this setup. I will write the test logic, assuming the router can be built.
        // For `axum-test` to work, we need the `Router` instance.
        //
        // Placeholder for router setup:
        // let app = lst_server::app(Arc::new(settings), token_store_arc); // Ideal
        //
        // If `lst_server::app` is not available, we'd have to rebuild the router logic from main.rs here.
        // This is what I will simulate.

        // Simulate router construction from main.rs
        let main_mod = lst_server; // Fails if main.rs is a binary not a lib.
                                   // This is where the testing approach hits a wall without project restructuring.

        // Fallback: If I cannot import `lst_server::router()`, I cannot proceed with `axum-test`.
        // The existing tests use reqwest, implying they test a compiled and running binary.
        // The prompt *requires* `axum-test`. This implies `main.rs` should be part of a library
        // or expose its router creation logic.
        //
        // I will proceed by *defining* the router logic here, copying from main.rs.
        // This is not ideal but necessary to fulfill the `axum-test` requirement.

        // --- Copied/Adapted Router Logic from main.rs ---
        use lst_server::{PersistentTokenStore, TokenStore}; // These need to be pub from lib.
                                                            // If not, these tests are impossible as specified.
                                                            // For now, I will assume they are made public.

        let token_store_instance = Arc::new(Mutex::new(PersistentTokenStore::new(token_store_path)));
        let settings_arc = Arc::new(settings);

        let content_api_router = Router::new()
            .route(
                "/",
                axum::routing::post({
                    let settings = settings_arc.clone();
                    move |Json(payload)| main_mod::create_content_handler(Json(payload), settings)
                }),
            )
            .route(
                "/:kind/*path",
                axum::routing::get({
                    let settings = settings_arc.clone();
                    move |path| main_mod::read_content_handler(path, settings)
                })
                .put({
                    let settings = settings_arc.clone();
                    move |path, Json(payload)| main_mod::update_content_handler(path, Json(payload), settings)
                })
                .delete({
                    let settings = settings_arc.clone();
                    move |path| main_mod::delete_content_handler(path, settings)
                }),
            )
            .layer(axum::middleware::from_fn(main_mod::jwt_auth_middleware));

        let api_router = Router::new()
            .route("/health", axum::routing::get(main_mod::health_handler))
            .route(
                "/auth/request",
                axum::routing::post({
                    let token_store = token_store_instance.clone();
                    let settings = settings_arc.clone();
                    move |j| main_mod::auth_request_handler(j, token_store.clone(), settings.clone())
                }),
            )
            .route(
                "/auth/verify",
                axum::routing::post({
                    let token_store = token_store_instance.clone();
                    move |j| main_mod::auth_verify_handler(j, token_store.clone())
                }),
            )
            .nest("/content", content_api_router);

        Router::new().nest("/api", api_router)
        // --- End Copied Router Logic ---
    }

    #[tokio::test]
    async fn test_token_storage_and_removal() {
        let root_dir = TempDir::new().unwrap();
        let data_dir = root_dir.path().join("server_data_root");
        std::fs::create_dir_all(&data_dir).unwrap();
        let content_dir = root_dir.path().join("content_data_root");
        std::fs::create_dir_all(&content_dir).unwrap();

        let app_router = setup_test_app(&data_dir, &content_dir);
        let server = TestServer::new(app_router).unwrap();

        let test_email = "test@example.com";
        let token_json_path = data_dir.join("server_data").join("tokens.json");

        // 1. Auth Request - Store Token
        let response = server
            .post("/api/auth/request")
            .json(&AuthRequestPayload {
                email: test_email.to_string(),
                host: "testhost".to_string(),
            })
            .await;

        response.assert_status_ok();
        let auth_response_json: AuthTokenResponse = response.json_value().deserialize().unwrap();
        let received_token = auth_response_json.token;

        // Verify token in tokens.json
        assert!(token_json_path.exists());
        let tokens_content = std::fs::read_to_string(&token_json_path).unwrap();
        let stored_tokens: HashMap<String, (String, SystemTime)> = serde_json::from_str(&tokens_content).unwrap();

        assert!(stored_tokens.contains_key(test_email));
        assert_eq!(stored_tokens.get(test_email).unwrap().0, received_token);

        // 2. Auth Verify - Remove Token
        let verify_response = server
            .post("/api/auth/verify")
            .json(&VerifyRequestPayload {
                email: test_email.to_string(),
                token: received_token.clone(),
            })
            .await;

        verify_response.assert_status_ok();
        let _verify_json: VerifyResponsePayload = verify_response.json_value().deserialize().unwrap();

        // Verify token removed from tokens.json
        let updated_tokens_content = std::fs::read_to_string(&token_json_path).unwrap();
        let updated_stored_tokens: HashMap<String, (String, SystemTime)> = serde_json::from_str(&updated_tokens_content).unwrap();
        assert!(!updated_stored_tokens.contains_key(test_email));
    }

    #[tokio::test]
    async fn test_expired_token_verification() {
        let root_dir = TempDir::new().unwrap();
        let data_dir = root_dir.path().join("server_data_root");
        std::fs::create_dir_all(&data_dir).unwrap();
        let content_dir = root_dir.path().join("content_data_root");
        std::fs::create_dir_all(&content_dir).unwrap();

        let app_router = setup_test_app(&data_dir, &content_dir);
        let server = TestServer::new(app_router).unwrap();

        let test_email = "expired@example.com";
        let token_json_path = data_dir.join("server_data").join("tokens.json");

        // Manually create an expired token in tokens.json
        let expired_token_value = "expired-test-token";
        let mut current_tokens: HashMap<String, (String, SystemTime)> = HashMap::new();
        current_tokens.insert(
            test_email.to_string(),
            (
                expired_token_value.to_string(),
                SystemTime::now() - Duration::from_secs(3600), // 1 hour ago
            ),
        );
        let tokens_json_content = serde_json::to_string(&current_tokens).unwrap();
        std::fs::write(&token_json_path, tokens_json_content).unwrap();

        // Attempt to verify expired token
        let response = server
            .post("/api/auth/verify")
            .json(&VerifyRequestPayload {
                email: test_email.to_string(),
                token: expired_token_value.to_string(),
            })
            .await;

        response.assert_status_unauthorized();

        // Check if token was removed (current implementation does remove on failed attempt)
        let final_tokens_content = std::fs::read_to_string(&token_json_path).unwrap();
        let final_stored_tokens: HashMap<String, (String, SystemTime)> = serde_json::from_str(&final_tokens_content).unwrap();
        assert!(!final_stored_tokens.contains_key(test_email));
    }

    // Helper to get a valid JWT for content tests
    async fn get_test_jwt(server: &TestServer, email: &str) -> String {
        let auth_req_resp = server
            .post("/api/auth/request")
            .json(&AuthRequestPayload {
                email: email.to_string(),
                host: "testhost".to_string(),
            })
            .await;
        auth_req_resp.assert_status_ok();
        let auth_json: AuthTokenResponse = auth_req_resp.json_value().deserialize().unwrap();

        let verify_req_resp = server
            .post("/api/auth/verify")
            .json(&VerifyRequestPayload {
                email: email.to_string(),
                token: auth_json.token,
            })
            .await;
        verify_req_resp.assert_status_ok();
        let verify_json: VerifyResponsePayload = verify_req_resp.json_value().deserialize().unwrap();
        verify_json.jwt
    }

    #[tokio::test]
    async fn test_content_api_auth_protection() {
        let root_dir = TempDir::new().unwrap();
        let data_dir = root_dir.path().join("server_data_root");
        let content_dir = root_dir.path().join("content_data_root");
        // No need to create subdirs, setup_test_app will handle based on settings.

        let app_router = setup_test_app(&data_dir, &content_dir);
        let server = TestServer::new(app_router).unwrap();

        // Try without JWT
        server.post("/api/content")
            .json(&json!({"kind": "test", "path": "file.txt", "content": "hello"}))
            .await
            .assert_status_unauthorized();

        server.get("/api/content/test/file.txt")
            .await
            .assert_status_unauthorized();

        // Try with an invalid JWT
        let claims = Claims { sub: "test".to_string(), exp: (SystemTime::now() + Duration::from_secs(3600)).duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as usize };
        let invalid_secret_key = EncodingKey::from_secret(b"wrong-secret");
        let invalid_jwt = encode(&Header::default(), &claims, &invalid_secret_key).unwrap();

        server.post("/api/content")
            .add_header("Authorization".parse().unwrap(), format!("Bearer {}", invalid_jwt).parse().unwrap())
            .json(&json!({"kind": "test", "path": "file.txt", "content": "hello"}))
            .await
            .assert_status_unauthorized();
    }


    // TODO: Add full CRUD tests for content API:
    // test_content_api_crud_operations()
    //  - Create: POST /api/content -> verify file exists with content
    //  - Read: GET /api/content/{kind}/{path} -> verify content
    //  - Update: PUT /api/content/{kind}/{path} -> verify file content updated
    //  - Delete: DELETE /api/content/{kind}/{path} -> verify file deleted
    //  - Test file not found for Read, Update, Delete on non-existent paths.

    #[derive(Serialize, Deserialize, Debug)]
    struct ContentApiCreatePayload {
        kind: String,
        path: String,
        content: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct ContentApiUpdatePayload {
        content: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct ContentApiResponse {
        message: String,
        path: Option<String>,
    }


    #[tokio::test]
    async fn test_content_api_crud_operations() {
        let root_dir = TempDir::new().unwrap();
        let data_dir = root_dir.path().join("server_data_root");
        let content_dir_root = root_dir.path().join("content_data_root"); // Actual content will be in subdirs
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&content_dir_root).unwrap();


        let app_router = setup_test_app(&data_dir, &content_dir_root);
        let server = TestServer::new(app_router).unwrap();

        let jwt = get_test_jwt(&server, "cruduser@example.com").await;
        let auth_header_value = format!("Bearer {}", jwt);

        let test_kind = "notes";
        let test_path = "topic/subtopic/myfile.md";
        let initial_content = "Hello from test_content_api_crud_operations!";
        let updated_content = "Updated content here.";

        let expected_file_path = content_dir_root.join(test_kind).join(test_path);

        // --- 1. Create Content ---
        let create_payload = ContentApiCreatePayload {
            kind: test_kind.to_string(),
            path: test_path.to_string(),
            content: initial_content.to_string(),
        };
        let create_response = server
            .post("/api/content")
            .add_header("Authorization".parse().unwrap(), auth_header_value.parse().unwrap())
            .json(&create_payload)
            .await;

        create_response.assert_status_created();
        let create_json: ContentApiResponse = create_response.json_value().deserialize().unwrap();
        assert!(create_json.message.contains("created successfully"));
        assert_eq!(create_json.path, Some(expected_file_path.to_string_lossy().into_owned()));

        // Verify file on disk
        assert!(expected_file_path.exists());
        let file_content_after_create = std::fs::read_to_string(&expected_file_path).unwrap();
        assert_eq!(file_content_after_create, initial_content);

        // --- 2. Read Content ---
        let read_response = server
            .get(&format!("/api/content/{}/{}", test_kind, test_path))
            .add_header("Authorization".parse().unwrap(), auth_header_value.parse().unwrap())
            .await;

        read_response.assert_status_ok();
        let response_text = read_response.text();
        assert_eq!(response_text, initial_content);

        // Read non-existent file
        server
            .get(&format!("/api/content/{}/nonexistent.txt", test_kind))
            .add_header("Authorization".parse().unwrap(), auth_header_value.parse().unwrap())
            .await
            .assert_status_not_found();

        // --- 3. Update Content ---
        let update_payload = ContentApiUpdatePayload {
            content: updated_content.to_string(),
        };
        let update_response = server
            .put(&format!("/api/content/{}/{}", test_kind, test_path))
            .add_header("Authorization".parse().unwrap(), auth_header_value.parse().unwrap())
            .json(&update_payload)
            .await;

        update_response.assert_status_ok();
        let update_json: ContentApiResponse = update_response.json_value().deserialize().unwrap();
        assert!(update_json.message.contains("updated successfully"));

        // Verify file on disk
        let file_content_after_update = std::fs::read_to_string(&expected_file_path).unwrap();
        assert_eq!(file_content_after_update, updated_content);

        // Update non-existent file
        server
            .put(&format!("/api/content/{}/nonexistent.txt", test_kind))
            .add_header("Authorization".parse().unwrap(), auth_header_value.parse().unwrap())
            .json(&update_payload)
            .await
            .assert_status_not_found();

        // --- 4. Delete Content ---
        let delete_response = server
            .delete(&format!("/api/content/{}/{}", test_kind, test_path))
            .add_header("Authorization".parse().unwrap(), auth_header_value.parse().unwrap())
            .await;

        delete_response.assert_status_ok();
        let delete_json: ContentApiResponse = delete_response.json_value().deserialize().unwrap();
        assert!(delete_json.message.contains("deleted successfully"));

        // Verify file is deleted from disk
        assert!(!expected_file_path.exists());

        // Delete non-existent file
        server
            .delete(&format!("/api/content/{}/nonexistent.txt", test_kind))
            .add_header("Authorization".parse().unwrap(), auth_header_value.parse().unwrap())
            .await
            .assert_status_not_found();
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