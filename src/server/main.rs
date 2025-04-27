mod wordlist;

use axum::{routing::{get, post}, Router, Json};
use axum::http::StatusCode;
use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex}};
use serde::{Deserialize, Serialize};
use rand::seq::SliceRandom;
use rand::Rng;
use qrcode::QrCode;
use image::Luma;
use base64::{engine::general_purpose, Engine as _};
use jsonwebtoken::{encode, Header, EncodingKey};

// Global in-memory token store (email -> (token, expiry))
type TokenMap = Arc<Mutex<HashMap<String, (String, std::time::Instant)>>>;
const TOKEN_VALID_FOR: u64 = 15 * 60; // 15 min in seconds

#[derive(Deserialize)]
struct AuthRequest {
    email: String,
    host: String // must be provided by the client for correct QR
}

#[derive(Serialize)]
struct AuthToken {
    token: String,
    qr_png_base64: String,
    login_url: String,
}

#[tokio::main]
async fn main() {
    let token_map: TokenMap = Arc::new(Mutex::new(HashMap::new()));
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/auth/request", post({
            let token_map = token_map.clone();
            move |j| auth_request_handler(j, token_map)
        }))
        .route("/auth/verify", post({
            let token_map = token_map.clone();
            move |j| auth_verify_handler(j, token_map)
        }));
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("lst-server listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn auth_request_handler(Json(req): Json<AuthRequest>, tokens: TokenMap) -> Json<AuthToken> {
    // Generate human token
    let token = generate_token();
    // Store token in the map with expiry
    let expiry = std::time::Instant::now() + std::time::Duration::from_secs(TOKEN_VALID_FOR);
    {
        let mut map = tokens.lock().unwrap();
        map.insert(req.email.clone(), (token.clone(), expiry));
    }
    // Build login url
    let login_url = format!(
        "lst-login://{}/auth/verify?token={}&email={}",
        req.host, urlencoding::encode(&token), urlencoding::encode(&req.email)
    );
    // Generate QR code (for URL)
    let code = QrCode::new(login_url.as_bytes()).unwrap();
    let img = code.render::<Luma<u8>>().max_dimensions(300, 300).build();
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        use image::codecs::png::PngEncoder;
        use image::ColorType;
        use image::ImageEncoder;
        let encoder = PngEncoder::new(&mut buf);
        let bytes = img.as_raw();
        encoder.write_image(
            bytes,
            img.width(),
            img.height(),
            ColorType::L8.into(),
        ).unwrap();
    }
    let base64_png = general_purpose::STANDARD.encode(buf.get_ref());
    
    Json(AuthToken {
        token,
        qr_png_base64: base64_png,
        login_url,
    })
}

fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let words = wordlist::WORDS;
    let picks: Vec<&str> = words.choose_multiple(&mut rng, 3).cloned().collect();
    let digits: u16 = rng.gen_range(1000..10000);
    format!("{}-{}-{}-{}", picks[0].to_uppercase(), picks[1].to_uppercase(), picks[2].to_uppercase(), digits)
}

#[derive(Deserialize)]
struct VerifyRequest {
    email: String,
    token: String,
}

#[derive(Serialize)]
struct VerifyResponse {
    jwt: String,
    user: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

async fn auth_verify_handler(Json(req): Json<VerifyRequest>, tokens: TokenMap) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    let now = std::time::Instant::now();
    let mut map = tokens.lock().unwrap();
    // Check token matches for email
    match map.remove(&req.email) {
        Some((t, expiry)) if t == req.token && expiry >= now => {
            // Generate JWT
            let jwt_secret = b"lst-jwt-demo-secret-goes-here";
            let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
            let claims = Claims { sub: req.email.clone(), exp };
            let jwt = encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret)).unwrap();
            Ok(Json(VerifyResponse { jwt, user: req.email }))
        }
        _ => Err((StatusCode::UNAUTHORIZED, "Invalid or expired token".into()))
    }
}
