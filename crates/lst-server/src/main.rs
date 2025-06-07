mod config;
mod wordlist;

use axum::{
    extract::Request,
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router, extract::Path,
};
use base64::{engine::general_purpose, Engine as _};
use clap::Parser;
use config::Settings;
use image::Luma;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{message::Message, AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
use std::path::Path as StdPath; // To disambiguate from axum::extract::Path
use tokio::fs;
use qrcode::QrCode;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

// --- Content Management Structs ---
#[derive(Deserialize)]
struct CreateContentRequest {
    kind: String,
    path: String, // e.g., "notes/topic/subtopic/file.md" or "image.png"
    content: String, // For text files, could be base64 for binary
}

#[derive(Deserialize)]
struct UpdateContentRequest {
    content: String, // For text files, could be base64 for binary
}

#[derive(Serialize)]
struct ContentResponse {
    message: String,
    path: Option<String>,
}

// Define the persistent token store
#[derive(Debug, Serialize, Deserialize)]
struct PersistentTokenStore {
    file_path: PathBuf,
    tokens: HashMap<String, (String, SystemTime)>,
}

impl PersistentTokenStore {
    fn new(mut data_path: PathBuf) -> Self {
        if !data_path.exists() {
            std::fs::create_dir_all(&data_path).expect("Failed to create data directory");
        }
        data_path.push("tokens.json");

        let mut store = PersistentTokenStore {
            file_path: data_path,
            tokens: HashMap::new(),
        };
        if let Err(e) = store.load() {
            eprintln!("Warning: Could not load tokens from {}: {}. Starting with an empty store.", store.file_path.display(), e);
            // If loading fails (e.g. file not found, corrupted), we start with an empty map,
            // and the first save operation will create/overwrite the file.
        }
        store
    }

    fn load(&mut self) -> Result<(), anyhow::Error> {
        if !self.file_path.exists() {
            // If the file doesn't exist, there's nothing to load.
            // A new file will be created on the first save.
            return Ok(());
        }
        let mut file = File::open(&self.file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        if contents.trim().is_empty() {
            // File is empty, treat as no tokens
            self.tokens = HashMap::new();
            return Ok(());
        }
        self.tokens = serde_json::from_str(&contents)?;
        Ok(())
    }

    fn save(&self) -> Result<(), anyhow::Error> {
        let contents = serde_json::to_string_pretty(&self.tokens)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true) // Overwrite the file if it exists
            .open(&self.file_path)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }

    fn insert(&mut self, email: String, token_value: String, expiry: SystemTime) -> Result<(), anyhow::Error> {
        self.tokens.insert(email, (token_value, expiry));
        self.save()
    }

    // Note: The original TokenMap used `remove` which returns the value.
    // This version will get, then remove if present, then save.
    // Or, more simply, just remove and then save. The return value can be constructed.
    fn remove(&mut self, email: &str) -> Option<(String, SystemTime)> {
        let removed_token = self.tokens.remove(email);
        if removed_token.is_some() {
            if let Err(e) = self.save() {
                eprintln!("Error saving tokens after remove: {}", e);
                // Decide if we should add the token back or how to handle this error.
                // For now, we'll proceed as if the save was successful, but log the error.
            }
        }
        removed_token
    }

    // Added a get method for convenience, though not strictly required by the prompt
    // if verify directly uses remove.
    #[allow(dead_code)]
    fn get(&self, email: &str) -> Option<&(String, SystemTime)> {
        self.tokens.get(email)
    }
}

// Type alias for the shared, thread-safe token store
type TokenStore = Arc<Mutex<PersistentTokenStore>>;
const TOKEN_VALID_FOR_SECS: u64 = 15 * 60; // 15 min in seconds
const JWT_SECRET: &[u8] = b"lst-jwt-demo-secret-goes-here";

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
    let config_file_path_str = if args.config.starts_with("~/") {
        dirs::home_dir().unwrap().join(&args.config[2..])
    } else {
        std::path::PathBuf::from(args.config)
    };
    let settings = Arc::new(Settings::from_file(&config_file_path_str).unwrap());

    // Determine data directory: use directory of config file + /server_data/
    let mut data_dir = config_file_path_str.clone();
    data_dir.pop(); // Remove filename to get directory
    data_dir.push("lst_server_data"); // Create a dedicated subdir for this server's data

    let token_store: TokenStore = Arc::new(Mutex::new(PersistentTokenStore::new(data_dir)));
    let settings_clone_for_content_routes = settings.clone();

    // Router for content API (protected)
    let content_api_router = Router::new()
        .route(
            "/", // Corresponds to /api/content
            post({
                let settings = settings_clone_for_content_routes.clone();
                move |Json(payload)| create_content_handler(Json(payload), settings)
            }),
        )
        .route(
            "/:kind/*path", // Corresponds to /api/content/:kind/*path
            get({
                let settings = settings_clone_for_content_routes.clone();
                move |path| read_content_handler(path, settings)
            })
            .put({
                let settings = settings_clone_for_content_routes.clone();
                move |path, Json(payload)| update_content_handler(path, Json(payload), settings)
            })
            .delete({
                let settings = settings_clone_for_content_routes.clone();
                move |path| delete_content_handler(path, settings)
            }),
        )
        .layer(middleware::from_fn(jwt_auth_middleware));

    // Main API router
    let api_router = Router::new()
        .route("/health", get(health_handler))
        .route(
            "/auth/request",
            post({
                let token_store = token_store.clone();
                let settings = settings.clone();
                move |j| auth_request_handler(j, token_store.clone(), settings.clone())
            }),
        )
        .route(
            "/auth/verify",
            post({
                let token_store = token_store.clone();
                move |j| auth_verify_handler(j, token_store.clone())
            }),
        )
        .nest("/content", content_api_router); // Nest the protected content router

    let app = Router::new().nest("/api", api_router);

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
    token_store: TokenStore,
    settings: Arc<Settings>,
) -> Result<Json<AuthToken>, (StatusCode, String)> {
    let token = generate_token();
    let expiry = SystemTime::now() + Duration::from_secs(TOKEN_VALID_FOR_SECS);
    {
        let mut store = token_store.lock().unwrap();
        if let Err(e) = store.insert(req.email.clone(), token.clone(), expiry) {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to store token: {}", e),
            ));
        }
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
    token_store: TokenStore,
) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    let now = SystemTime::now();
    let mut store = token_store.lock().unwrap();
    // Check token matches for email
    match store.remove(&req.email) {
        Some((t, expiry)) if t == req.token && expiry >= now => {
            // Generate JWT
            let exp = (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize;
            let claims = Claims {
                sub: req.email.clone(),
                exp,
            };
            let jwt = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(JWT_SECRET),
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

// --- Content Management Helper Functions ---

fn get_content_base_path(settings: &Arc<Settings>) -> Result<PathBuf, (StatusCode, String)> {
    match &settings.paths.content_dir {
        Some(cd_str) => {
            let mut path = PathBuf::new();
            if cd_str.starts_with("~/") {
                match dirs::home_dir() {
                    Some(home) => path.push(home.join(&cd_str[2..])),
                    None => return Err((StatusCode::INTERNAL_SERVER_ERROR, "Could not determine home directory for content path".to_string())),
                }
            } else {
                path.push(cd_str);
            }
            Ok(path)
        }
        None => Err((
            StatusCode::BAD_REQUEST,
            "Content directory not configured in server settings.".to_string(),
        )),
    }
}

// Helper to build and sanitize the content path
fn build_content_path(
    base_content_dir: &PathBuf,
    kind: &str,
    item_path: &str,
) -> Result<PathBuf, (StatusCode, String)> {
    if kind.contains("..") || kind.contains('/') || kind.contains('\\') {
        return Err((StatusCode::BAD_REQUEST, "Invalid 'kind' parameter.".to_string()));
    }
    if item_path.contains("..") { // A more robust check would be to check each component
        return Err((StatusCode::BAD_REQUEST, "Invalid 'path' parameter (contains '..').".to_string()));
    }

    let mut full_path = base_content_dir.clone();
    full_path.push(kind);
    full_path.push(item_path.trim_start_matches('/')); // Ensure item_path is relative

    // Normalize the path (e.g. remove foo/./bar, resolve foo/../bar)
    // and ensure it's still within base_content_dir.
    // std::fs::canonicalize can be used but it requires path to exist.
    // For now, we rely on the '..' check and the fact that we construct it by joining.
    // A more robust sanitization might involve checking components of the path.

    Ok(full_path)
}


// --- Content Management Handlers ---

async fn create_content_handler(
    Json(payload): Json<CreateContentRequest>,
    settings: Arc<Settings>,
) -> Result<(StatusCode, Json<ContentResponse>), (StatusCode, String)> {
    let base_path = get_content_base_path(&settings)?;
    let file_path = build_content_path(&base_path, &payload.kind, &payload.path)?;

    if let Some(parent_dir) = file_path.parent() {
        fs::create_dir_all(parent_dir)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create directory: {}", e)))?;
    }

    fs::write(&file_path, &payload.content)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write file: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(ContentResponse {
            message: "Content created successfully.".to_string(),
            path: Some(file_path.to_string_lossy().into_owned()),
        }),
    ))
}

async fn read_content_handler(
    Path((kind, path)): Path<(String, String)>,
    settings: Arc<Settings>,
) -> Result<String, (StatusCode, String)> {
    let base_path = get_content_base_path(&settings)?;
    let file_path = build_content_path(&base_path, &kind, &path)?;

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Content not found.".to_string()));
    }

    fs::read_to_string(&file_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read file: {}", e)))
}

async fn update_content_handler(
    Path((kind, path)): Path<(String, String)>,
    Json(payload): Json<UpdateContentRequest>,
    settings: Arc<Settings>,
) -> Result<Json<ContentResponse>, (StatusCode, String)> {
    let base_path = get_content_base_path(&settings)?;
    let file_path = build_content_path(&base_path, &kind, &path)?;

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Content not found. Use POST to create.".to_string()));
    }

    fs::write(&file_path, &payload.content)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update file: {}", e)))?;

    Ok(Json(ContentResponse {
        message: "Content updated successfully.".to_string(),
        path: Some(file_path.to_string_lossy().into_owned()),
    }))
}

// --- JWT Auth Middleware ---
async fn jwt_auth_middleware<B>(
    req: Request<B>, // Changed from headers: HeaderMap to full request
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let headers = req.headers();
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            let decoding_key = DecodingKey::from_secret(JWT_SECRET);
            // TODO: Potentially make validation more configurable if needed (e.g. audience, issuer)
            let validation = Validation::default();

            match decode::<Claims>(token, &decoding_key, &validation) {
                Ok(_token_data) => {
                    // Token is valid, proceed to the next handler
                    // Optionally, pass claims via request extensions:
                    // req.extensions_mut().insert(token_data.claims);
                    return Ok(next.run(req).await);
                }
                Err(e) => {
                    eprintln!("JWT validation error: {}", e);
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

async fn delete_content_handler(
    Path((kind, path)): Path<(String, String)>,
    settings: Arc<Settings>,
) -> Result<Json<ContentResponse>, (StatusCode, String)> {
    let base_path = get_content_base_path(&settings)?;
    let file_path = build_content_path(&base_path, &kind, &path)?;

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Content not found.".to_string()));
    }

    fs::remove_file(&file_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete file: {}", e)))?;

    Ok(Json(ContentResponse {
        message: "Content deleted successfully.".to_string(),
        path: Some(file_path.to_string_lossy().into_owned()),
    }))
}
