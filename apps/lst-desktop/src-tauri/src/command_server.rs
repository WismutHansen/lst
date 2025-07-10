use axum::{routing::post, Router};
use std::net::SocketAddr;
use tauri::{AppHandle, Emitter, Manager};
use tower_http::cors::{Any, CorsLayer};

async fn test_handler(app_handle: AppHandle) {
    println!("Test endpoint called");
    match app_handle.emit("test-event", "Hello from backend!") {
        Ok(_) => println!("󰸞 Test event emitted successfully"),
        Err(e) => println!(" Failed to emit test event: {}", e),
    }
}

async fn switch_list_handler(app_handle: AppHandle, list_name: String) {
    println!("🔄 CLI command received: switching to list '{}'", list_name);

    // // Try emitting globally
    // match app_handle.emit("switch-list", &list_name) {
    //     Ok(_) => println!(
    //         "󰸞 Event 'switch-list' emitted globally with payload: '{}'",
    //         list_name
    //     ),
    //     Err(e) => println!(" Failed to emit 'switch-list' event globally: {}", e),
    // }
    //
    // Also try emitting to main window specifically
    if let Some(window) = app_handle.get_webview_window("main") {
        match window.emit("switch-list", &list_name) {
            Ok(_) => println!(
                "󰸞 Event 'switch-list' emitted to main window with payload: '{}'",
                list_name
            ),
            Err(e) => println!(" Failed to emit 'switch-list' event to main window: {}", e),
        }
    } else {
        println!(" Could not find main window");
    }
}

async fn show_message_handler(app_handle: AppHandle, message: String) {
    println!("💬 CLI command received: showing message '{}'", message);

    if let Some(window) = app_handle.get_webview_window("main") {
        match window.emit("show-message", &message) {
            Ok(_) => println!(
                "󰸞 Event 'show-message' emitted to main window with payload: '{}'",
                message
            ),
            Err(e) => println!(" Failed to emit 'show-message' event to main window: {}", e),
        }
    } else {
        println!(" Could not find main window");
    }
}

pub fn start_command_server(app_handle: AppHandle) {
    println!("🚀 Starting command server...");
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);

            let app_handle_1 = app_handle.clone();
            let app_handle_2 = app_handle.clone();
            let app_handle_3 = app_handle.clone();

            let app = Router::new()
                .route(
                    "/command/switch-list",
                    post(move |list_name: String| {
                        switch_list_handler(app_handle_1.clone(), list_name)
                    }),
                )
                .route(
                    "/command/show-message",
                    post(move |message: String| {
                        show_message_handler(app_handle_2.clone(), message)
                    }),
                )
                .route(
                    "/command/test",
                    post(move |_: String| test_handler(app_handle_3.clone())),
                )
                .layer(cors);

            let addr = SocketAddr::from(([127, 0, 0, 1], 33333));
            println!("🎯 Binding command server to {}", addr);
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            println!("✅ Command server listening on http://{}", addr);
            axum::serve(listener, app).await.unwrap();
        });
    });
}
