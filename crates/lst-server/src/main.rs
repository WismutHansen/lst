mod config;
mod sync_db;
mod wordlist;

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        Path, Request, State,
    },
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use config::Settings;
use futures_util::{SinkExt, StreamExt};
use hex;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{
    message::Message as EmailMessage, AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use qrcode::render::unicode;
use qrcode::QrCode;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use argon2::{password_hash::{SaltString, PasswordHash}, Argon2, Algorithm, Params, PasswordHasher, PasswordVerifier, Version};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::{FromRow, Row};
use std::net::{IpAddr, SocketAddr};
use std::path::Path as StdPath;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::broadcast;

// --- Structs for API Payloads and Responses ---
#[derive(Deserialize)]
struct CreateContentRequest {
    kind: String,
    path: String,
    content: String,
}

#[derive(Deserialize)]
struct UpdateContentRequest {
    content: String,
}

#[derive(Serialize)]
struct ContentResponse {
    message: String,
    path: Option<String>,
}

// --- SQLite Token Store ---
#[derive(Debug, Clone)]
pub struct SqliteTokenStore {
    pool: SqlitePool,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct StoredToken {
    email: String,
    token_value: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl SqliteTokenStore {
    pub async fn new(db_path: PathBuf) -> Result<Self, sqlx::Error> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
                })?;
            }
        }
        let db_url = format!("sqlite://{}?mode=rwc", db_path.to_str().unwrap());
        eprintln!(
            "DEBUG: Attempting to connect to tokens database at: {}",
            db_url
        );
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tokens (
                email TEXT PRIMARY KEY NOT NULL,
                token_value TEXT NOT NULL,
                expires_at TIMESTAMP NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                email TEXT PRIMARY KEY NOT NULL,
                password_hash TEXT NOT NULL,
                salt TEXT NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;
        Ok(SqliteTokenStore { pool })
    }

    pub async fn insert(
        &self,
        email: String,
        token: String,
        expires_at: SystemTime,
    ) -> Result<(), sqlx::Error> {
        let expires_at_chrono: chrono::DateTime<chrono::Utc> = expires_at.into();
        sqlx::query(
            "INSERT OR REPLACE INTO tokens (email, token_value, expires_at) VALUES (?, ?, ?)",
        )
        .bind(email)
        .bind(token)
        .bind(expires_at_chrono)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn verify_and_remove(
        &self,
        email: &str,
        token_to_check: &str,
    ) -> Result<bool, sqlx::Error> {
        let result: Option<StoredToken> =
            sqlx::query_as("SELECT email, token_value, expires_at FROM tokens WHERE email = ?")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?;
        match result {
            Some(stored_token) => {
                let expires_at_system: SystemTime = stored_token.expires_at.into();
                let is_valid = stored_token.token_value == token_to_check
                    && expires_at_system > SystemTime::now();
                sqlx::query("DELETE FROM tokens WHERE email = ?")
                    .bind(email)
                    .execute(&self.pool)
                    .await?;
                Ok(is_valid)
            }
            None => Ok(false),
        }
    }

    pub async fn get_user(&self, email: &str) -> Result<Option<(String, String)>, sqlx::Error> {
        if let Some(row) = sqlx::query("SELECT password_hash, salt FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?
        {
            let password_hash: String = row.get(0);
            let salt: String = row.get(1);
            Ok(Some((password_hash, salt)))
        } else {
            Ok(None)
        }
    }

    pub async fn set_user(
        &self,
        email: &str,
        password_hash: &str,
        salt: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT OR REPLACE INTO users (email, password_hash, salt) VALUES (?, ?, ?)")
            .bind(email)
            .bind(password_hash)
            .bind(salt)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

type TokenStore = Arc<SqliteTokenStore>;
const TOKEN_VALID_FOR_SECS: u64 = 15 * 60;
const JWT_SECRET: &[u8] = b"lst-jwt-demo-secret-goes-here";

// --- SQLite Content Store ---
#[derive(Debug, Clone)]
pub struct SqliteContentStore {
    pool: SqlitePool,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct ContentRow {
    #[allow(dead_code)]
    id: i64,
    kind: String,
    item_path: String, // Renamed from 'path' to avoid confusion
    content: String,
    #[allow(dead_code)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[allow(dead_code)]
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl SqliteContentStore {
    pub async fn new(db_path: PathBuf) -> Result<Self, sqlx::Error> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    sqlx::Error::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
                })?;
            }
        }
        let db_url = format!("sqlite://{}?mode=rwc", db_path.to_str().unwrap());
        eprintln!(
            "DEBUG: Attempting to connect to content database at: {}",
            db_url
        );

        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .connect(&db_url)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS content (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                item_path TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (kind, item_path)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Trigger to automatically update `updated_at`
        sqlx::query(
            r#"
            CREATE TRIGGER IF NOT EXISTS content_auto_update_updated_at
            AFTER UPDATE ON content
            FOR EACH ROW
            WHEN OLD.content IS NOT NEW.content OR OLD.item_path IS NOT NEW.item_path OR OLD.kind IS NOT NEW.kind
            BEGIN
                UPDATE content SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
            END;
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(SqliteContentStore { pool })
    }

    pub async fn create_content(
        &self,
        kind: &str,
        item_path: &str,
        content: &str,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO content (kind, item_path, content)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(kind)
        .bind(item_path)
        .bind(content)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn read_content(
        &self,
        kind: &str,
        item_path: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let result: Option<sqlx::sqlite::SqliteRow> = sqlx::query(
            r#"
            SELECT content FROM content WHERE kind = ? AND item_path = ?
            "#,
        )
        .bind(kind)
        .bind(item_path)
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.map(|row| row.get("content")))
    }

    pub async fn update_content(
        &self,
        kind: &str,
        item_path: &str,
        content: &str,
    ) -> Result<u64, sqlx::Error> {
        // The trigger will handle updated_at if the content actually changes.
        // If only other fields were to change, we might need explicit updated_at here.
        // For this case, content is the main mutable part besides path/kind (which would be a new row).
        let result = sqlx::query(
            r#"
            UPDATE content SET content = ?
            WHERE kind = ? AND item_path = ?
            "#,
        )
        .bind(content)
        .bind(kind)
        .bind(item_path)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_content(&self, kind: &str, item_path: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM content WHERE kind = ? AND item_path = ?
            "#,
        )
        .bind(kind)
        .bind(item_path)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

type ContentStore = Arc<SqliteContentStore>;

#[derive(Clone)]
struct AppState {
    db: sync_db::SyncDb,
    tx: broadcast::Sender<(String, lst_proto::ServerMessage)>,
}

#[derive(Deserialize)]
struct AuthRequest {
    email: String,
    host: String,
    password_hash: String,
}

#[derive(Serialize)]
struct AuthResponse {
    status: String,
}

#[derive(Parser)]
#[command(name = "lst-server", about = "lst server API")]
struct Args {
    #[arg(long, default_value = "~/.config/lst/lst.toml")]
    config: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config_file_path_str = if args.config.starts_with("~/") {
        dirs::home_dir().unwrap().join(&args.config[2..])
    } else {
        StdPath::new(&args.config).to_path_buf()
    };
    let settings = Arc::new(Settings::from_file(&config_file_path_str).unwrap());

    // Get database paths from configuration
    let tokens_db_path = settings
        .database
        .tokens_db_path()
        .expect("Failed to resolve tokens database path");
    let content_db_path = settings
        .database
        .content_db_path()
        .expect("Failed to resolve content database path");
    let sync_db_path = settings
        .database
        .sync_db_path()
        .expect("Failed to resolve sync database path");

    let token_store = Arc::new(
        SqliteTokenStore::new(tokens_db_path)
            .await
            .expect("Failed to initialize token store"),
    );

    let content_store = Arc::new(
        SqliteContentStore::new(content_db_path)
            .await
            .expect("Failed to initialize content store"),
    );

    let sync_db = sync_db::SyncDb::new(sync_db_path)
        .await
        .expect("Failed to initialize sync db");
    let (tx, _) = broadcast::channel(100);
    let app_state = Arc::new(AppState { db: sync_db, tx });

    // Router for content API (protected)
    // The handlers (e.g., create_content_handler) will be updated next to accept ContentStore
    let content_api_router = Router::new()
        .route(
            "/",
            post({
                let store = content_store.clone();
                // Signature of create_content_handler will change from Arc<Settings> to ContentStore
                move |Json(payload)| create_content_handler(Json(payload), store)
            }),
        )
        .route(
            "/{kind}/{*path}",
            get({
                let store = content_store.clone();
                // Signature of read_content_handler will change
                move |path| read_content_handler(path, store)
            })
            .put({
                let store = content_store.clone();
                // Signature of update_content_handler will change
                move |path, Json(payload)| update_content_handler(path, Json(payload), store)
            })
            .delete({
                let store = content_store.clone();
                // Signature of delete_content_handler will change
                move |path| delete_content_handler(path, store)
            }),
        )
        .layer(middleware::from_fn(jwt_auth_middleware));

    let api_router =
        Router::new()
            .route("/health", get(health_handler))
            .route(
                "/auth/request",
                post({
                    let ts = token_store.clone();
                    let s = settings.clone();
                    move |j| auth_request_handler(j, ts, s)
                }),
            )
            .route(
                "/auth/verify",
                post({
                    let ts = token_store.clone();
                    move |j| auth_verify_handler(j, ts)
                }),
            )
            .nest("/content", content_api_router)
            .route(
                "/sync",
                get(
                    |ws: WebSocketUpgrade,
                     State(state): State<Arc<AppState>>,
                     headers: HeaderMap| async move {
                        ws_handler(ws, headers, State(state)).await
                    },
                ),
            );
    let app = Router::new()
        .nest("/api", api_router)
        .with_state(app_state.clone());

    let addr = SocketAddr::new(
        settings.server.host.parse::<IpAddr>().unwrap(),
        settings.server.port,
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
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    // verify or create user
    let params = Params::new(128 * 1024, 3, 2, None).expect("invalid params");
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    if let Ok(Some((stored, _salt))) = token_store.get_user(&req.email).await {
        let parsed = PasswordHash::new(&stored).map_err(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "Corrupt password hash".into())
        })?;
        if argon2
            .verify_password(req.password_hash.as_bytes(), &parsed)
            .is_err()
        {
            return Err((StatusCode::UNAUTHORIZED, "Invalid password".into()));
        }
    } else {
        let salt = SaltString::encode_b64(rand::random::<[u8; 16]>())
            .expect("salt");
        let final_hash = argon2
            .hash_password(req.password_hash.as_bytes(), &salt)
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to hash password".to_string(),
                )
            })?
            .to_string();
        token_store
            .set_user(&req.email, &final_hash, salt.as_str())
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to store user".to_string(),
                )
            })?;
    }
    let token = generate_token();
    let expiry = SystemTime::now() + Duration::from_secs(TOKEN_VALID_FOR_SECS);
    if let Err(e) = token_store
        .insert(req.email.clone(), token.clone(), expiry)
        .await
    {
        eprintln!("Failed to store token: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to process authentication request.".to_string(),
        ));
    }
    let login_url = format!(
        "lst-login://{}/auth/verify?token={}&email={}",
        req.host,
        urlencoding::encode(&token),
        urlencoding::encode(&req.email)
    );
    let code = QrCode::new(login_url.as_bytes()).unwrap();
    let qr_string = code.render::<unicode::Dense1x2>().build();
    if let Some(email_cfg) = &settings.email {
        let email_message = EmailMessage::builder()
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
        mailer.send(email_message).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to send email: {}", e),
            )
        })?;
    } else {
        println!("Authentication token for {}: {}", req.email, token);
        println!("Login link: {}", login_url);
        println!("\nScan the following QR code to log in:");
        println!("{}", qr_string);
    }
    Ok(Json(AuthResponse {
        status: "ok".to_string(),
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

#[derive(Serialize, Deserialize, Debug, Clone)] // Added Clone here for the middleware
struct Claims {
    sub: String,
    exp: usize,
}

async fn auth_verify_handler(
    Json(req): Json<VerifyRequest>,
    token_store: TokenStore,
) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    match token_store.verify_and_remove(&req.email, &req.token).await {
        Ok(true) => {
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
        Ok(false) | Err(_) => Err((StatusCode::UNAUTHORIZED, "Invalid or expired token".into())),
    }
}

// --- Content Management Handlers (SQLite based) ---

async fn create_content_handler(
    Json(payload): Json<CreateContentRequest>,
    store: ContentStore,
) -> Result<(StatusCode, Json<ContentResponse>), (StatusCode, String)> {
    // Basic validation for kind and path
    if payload.kind.is_empty()
        || payload.kind.contains('/')
        || payload.kind.contains("..")
        || payload.kind.starts_with('.')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid 'kind' parameter.".to_string(),
        ));
    }
    if payload.path.is_empty()
        || payload.path.contains("..")
        || payload.path.starts_with('/')
        || payload.path.ends_with('/')
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid 'path' parameter.".to_string(),
        ));
    }

    match store
        .create_content(&payload.kind, &payload.path, &payload.content)
        .await
    {
        Ok(_id) => Ok((
            StatusCode::CREATED,
            Json(ContentResponse {
                message: "Content created successfully.".to_string(),
                path: Some(format!("{}/{}", payload.kind, payload.path)), // Return logical path
            }),
        )),
        Err(e) => {
            if let Some(db_err) = e.as_database_error() {
                if db_err.is_unique_violation() {
                    return Err((
                        StatusCode::CONFLICT,
                        "Content with this kind and path already exists.".to_string(),
                    ));
                }
            }
            eprintln!("Failed to create content: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create content.".to_string(),
            ))
        }
    }
}

async fn read_content_handler(
    Path((kind, item_path)): Path<(String, String)>,
    store: ContentStore,
) -> Result<Response, (StatusCode, String)> {
    match store.read_content(&kind, &item_path).await {
        Ok(Some(content)) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                "text/plain; charset=utf-8".parse().unwrap(),
            );
            Ok((StatusCode::OK, headers, content).into_response())
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, "Content not found.".to_string())),
        Err(e) => {
            eprintln!("Failed to read content: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read content.".to_string(),
            ))
        }
    }
}

async fn update_content_handler(
    Path((kind, item_path)): Path<(String, String)>,
    Json(payload): Json<UpdateContentRequest>,
    store: ContentStore,
) -> Result<Json<ContentResponse>, (StatusCode, String)> {
    match store
        .update_content(&kind, &item_path, &payload.content)
        .await
    {
        Ok(affected_rows) => {
            if affected_rows > 0 {
                Ok(Json(ContentResponse {
                    message: "Content updated successfully.".to_string(),
                    path: Some(format!("{}/{}", kind, item_path)),
                }))
            } else {
                Err((StatusCode::NOT_FOUND, "Content not found.".to_string()))
            }
        }
        Err(e) => {
            eprintln!("Failed to update content: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update content.".to_string(),
            ))
        }
    }
}

async fn delete_content_handler(
    Path((kind, item_path)): Path<(String, String)>,
    store: ContentStore,
) -> Result<Json<ContentResponse>, (StatusCode, String)> {
    match store.delete_content(&kind, &item_path).await {
        Ok(affected_rows) => {
            if affected_rows > 0 {
                Ok(Json(ContentResponse {
                    message: "Content deleted successfully.".to_string(),
                    path: Some(format!("{}/{}", kind, item_path)),
                }))
            } else {
                Err((StatusCode::NOT_FOUND, "Content not found.".to_string()))
            }
        }
        Err(e) => {
            eprintln!("Failed to delete content: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete content.".to_string(),
            ))
        }
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    if let Some(auth) = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    {
        let decoding_key = DecodingKey::from_secret(JWT_SECRET);
        let validation = Validation::default();
        if let Ok(token_data) = decode::<Claims>(auth, &decoding_key, &validation) {
            let user = token_data.claims.sub;
            return ws.on_upgrade(move |socket| handle_ws(socket, state, user));
        }
    }
    (StatusCode::UNAUTHORIZED, "unauthorized").into_response()
}

async fn handle_ws(stream: WebSocket, state: Arc<AppState>, user: String) {
    let (mut sender, mut receiver) = stream.split();

    let _ = sender
        .send(WsMessage::Text(
            serde_json::to_string(&lst_proto::ServerMessage::Authenticated { success: true })
                .unwrap()
                .into(),
        ))
        .await;

    let user_clone = user.clone();
    let mut rx = state.tx.subscribe();
    let (tx, mut rx_local) = tokio::sync::mpsc::channel::<WsMessage>(100);

    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle broadcast messages
                Ok((target, msg)) = rx.recv() => {
                    if target == user_clone {
                        if let Ok(txt) = serde_json::to_string(&msg) {
                            if sender.send(WsMessage::Text(txt.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                // Handle direct messages from main task
                Some(msg) = rx_local.recv() => {
                    if sender.send(msg).await.is_err() {
                        break;
                    }
                }
                else => break,
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        if let WsMessage::Text(text) = msg {
            if let Ok(cmsg) = serde_json::from_str::<lst_proto::ClientMessage>(&text) {
                match cmsg {
                    lst_proto::ClientMessage::RequestDocumentList => {
                        if let Ok(list) = state.db.list_documents(&user).await {
                            let resp = lst_proto::ServerMessage::DocumentList { documents: list };
                            let _ = tx
                                .send(WsMessage::Text(
                                    serde_json::to_string(&resp).unwrap().into(),
                                ))
                                .await;
                        }
                    }
                    lst_proto::ClientMessage::RequestSnapshot { doc_id } => {
                        if let Ok(Some(snap)) = state.db.get_snapshot(&doc_id.to_string()).await {
                            let resp = lst_proto::ServerMessage::Snapshot {
                                doc_id,
                                snapshot: snap,
                            };
                            let _ = tx
                                .send(WsMessage::Text(
                                    serde_json::to_string(&resp).unwrap().into(),
                                ))
                                .await;
                        }
                    }
                    lst_proto::ClientMessage::PushChanges {
                        doc_id,
                        device_id,
                        changes,
                    } => {
                        let _ = state
                            .db
                            .add_changes(&doc_id.to_string(), &device_id, &changes)
                            .await;
                        let msg = lst_proto::ServerMessage::NewChanges {
                            doc_id,
                            from_device_id: device_id,
                            changes,
                        };
                        let _ = state.tx.send((user.clone(), msg));
                    }
                    lst_proto::ClientMessage::PushSnapshot { doc_id, snapshot } => {
                        let _ = state
                            .db
                            .save_snapshot(&doc_id.to_string(), &user, &snapshot)
                            .await;
                    }
                    lst_proto::ClientMessage::Authenticate { .. } => {}
                }
            }
        }
    }

    send_task.abort();
}

// --- JWT Auth Middleware ---
async fn jwt_auth_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let headers = req.headers();
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());
    if let Some(auth_header) = auth_header {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            let decoding_key = DecodingKey::from_secret(JWT_SECRET);
            let validation = Validation::default();
            match decode::<Claims>(token, &decoding_key, &validation) {
                Ok(_token_data) => {
                    // req.extensions_mut().insert(token_data.claims); // Example: pass claims
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
