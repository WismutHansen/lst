mod config;
mod wordlist;

use axum::http::StatusCode;
use axum::{
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use config::Settings;
use image::Luma;
use jsonwebtoken::{encode, EncodingKey, Header};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{message::Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use qrcode::QrCode;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

// Global in-memory token store (email -> (token, expiry))
type TokenMap = Arc<Mutex<HashMap<String, (String, std::time::Instant)>>>;
const TOKEN_VALID_FOR: u64 = 15 * 60; // 15 min in seconds

#[derive(Deserialize)]
struct AuthRequest {
    email: String,
    host: String, // must be provided by the client for correct QR
}

#[derive(Serialize)]
struct AuthToken {
    token: String,
    qr_png_base64: String,
    login_url: String,
}

#[derive(Parser)]
#[command(name = "lst-server", about = "lst server API")]
struct Args {
    /// Path to server configuration TOML file
    #[arg(long, default_value = "~/.config/lst/lst.toml")]
    config: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config_path = if args.config.starts_with("~/") {
        dirs::home_dir().unwrap().join(&args.config[2..])
    } else {
        std::path::PathBuf::from(args.config)
    };
    let settings = Arc::new(Settings::from_file(&config_path).unwrap());

    let token_map: TokenMap = Arc::new(Mutex::new(HashMap::new()));
    let app = Router::new().nest(
        "/api",
        Router::new()
            .route("/health", get(health_handler))
            .route(
                "/auth/request",
                post({
                    let token_map = token_map.clone();
                    let settings = settings.clone();
                    move |j| auth_request_handler(j, token_map.clone(), settings.clone())
                }),
            )
            .route(
                "/auth/verify",
                post({
                    let token_map = token_map.clone();
                    move |j| auth_verify_handler(j, token_map.clone())
                }),
            ),
    );
    let addr = SocketAddr::new(
        settings.lst_server.host.parse::<IpAddr>().unwrap(),
        settings.lst_server.port,
    );
    println!("lst-server listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn auth_request_handler(
    Json(req): Json<AuthRequest>,
    tokens: TokenMap,
    settings: Arc<Settings>,
) -> Result<Json<AuthToken>, (StatusCode, String)> {
    let token = generate_token();
    let expiry = std::time::Instant::now() + std::time::Duration::from_secs(TOKEN_VALID_FOR);
    {
        let mut map = tokens.lock().unwrap();
        map.insert(req.email.clone(), (token.clone(), expiry));
    }
    let login_url = format!(
        "lst-login://{}/auth/verify?token={}&email={}",
        req.host,
        urlencoding::encode(&token),
        urlencoding::encode(&req.email)
    );
    let code = QrCode::new(login_url.as_bytes()).unwrap();
    let img = code.render::<Luma<u8>>().max_dimensions(300, 300).build();
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        use image::codecs::png::PngEncoder;
        use image::ColorType;
        use image::ImageEncoder;

        let encoder = PngEncoder::new(&mut buf);
        encoder
            .write_image(
                img.as_raw(),
                img.width(),
                img.height(),
                ColorType::L8.into(),
            )
            .unwrap();
    }
    let base64_png = general_purpose::STANDARD.encode(buf.get_ref());

    if let Some(email_cfg) = &settings.email {
        let email = Message::builder()
            .from(email_cfg.sender.parse().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("invalid sender address: {}", e),
                )
            })?)
            .to(req.email.parse().map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("invalid recipient address: {}", e),
                )
            })?)
            .subject("Your lst login link")
            .body(format!(
                "Click to login: {}\nOr use code: {}",
                login_url, token
            ))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to build email: {}", e),
                )
            })?;
        let creds = Credentials::new(email_cfg.smtp_user.clone(), email_cfg.smtp_pass.clone());
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&email_cfg.smtp_host)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to create SMTP transport: {}", e),
                )
            })?
            .credentials(creds)
            .build();
        mailer.send(email).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to send email: {}", e),
            )
        })?;
    } else {
        println!("Login link for {}: {}", req.email, login_url);
    }

    Ok(Json(AuthToken {
        token,
        qr_png_base64: base64_png,
        login_url,
    }))
}

fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let words = wordlist::WORDS;
    let picks: Vec<&str> = words.choose_multiple(&mut rng, 3).cloned().collect();
    let digits: u16 = rng.gen_range(1000..10000);
    format!(
        "{}-{}-{}-{}",
        picks[0].to_uppercase(),
        picks[1].to_uppercase(),
        picks[2].to_uppercase(),
        digits
    )
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

async fn auth_verify_handler(
    Json(req): Json<VerifyRequest>,
    tokens: TokenMap,
) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    let now = std::time::Instant::now();
    let mut map = tokens.lock().unwrap();
    // Check token matches for email
    match map.remove(&req.email) {
        Some((t, expiry)) if t == req.token && expiry >= now => {
            // Generate JWT
            let jwt_secret = b"lst-jwt-demo-secret-goes-here";
            let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
            let claims = Claims {
                sub: req.email.clone(),
                exp,
            };
            let jwt = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(jwt_secret),
            )
            .unwrap();
            Ok(Json(VerifyResponse {
                jwt,
                user: req.email,
            }))
        }
        _ => Err((StatusCode::UNAUTHORIZED, "Invalid or expired token".into())),
    }
}
