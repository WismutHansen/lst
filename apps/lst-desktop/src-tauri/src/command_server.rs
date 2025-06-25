
use axum::{
    routing::post,
    Router,
};
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter};
use tower_http::cors::{Any, CorsLayer};

async fn switch_list_handler(app_handle: AppHandle, list_name: String) {
    app_handle.emit("switch-list", list_name).unwrap();
}

pub fn start_command_server(app_handle: AppHandle) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);

            let app = Router::new()
                .route(
                    "/command/switch-list",
                    post(move |list_name: String| switch_list_handler(app_handle.clone(), list_name)),
                )
                .layer(cors);

            let addr = SocketAddr::from(([127, 0, 0, 1], 33333));
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });
    });
}
