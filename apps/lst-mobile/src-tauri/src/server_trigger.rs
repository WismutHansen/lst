use crate::mobile_config::MobileConfig;
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::Duration;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug)]
pub enum MobileTriggerEvent {
    RemoteChange,
}

pub struct MobileServerTrigger {
    rx: UnboundedReceiver<MobileTriggerEvent>,
}

impl MobileServerTrigger {
    pub fn spawn(config: &MobileConfig) -> Option<Self> {
        let server_url = config.syncd.as_ref().and_then(|s| s.url.clone())?;
        let jwt = config.get_jwt()?.to_string();

        let (tx, rx) = unbounded_channel();
        tokio::spawn(run_listener(server_url, jwt, tx.clone()));

        Some(Self { rx })
    }

    pub async fn recv(&mut self) -> Option<MobileTriggerEvent> {
        self.rx.recv().await
    }
}

fn normalize_ws_url(server_url: &str) -> String {
    let mut ws_url = server_url
        .replace("http://", "ws://")
        .replace("https://", "wss://");

    if !ws_url.ends_with("/api/sync") {
        if !ws_url.ends_with('/') {
            ws_url.push('/');
        }
        ws_url.push_str("api/sync");
    }

    ws_url
}

async fn run_listener(server_url: String, jwt: String, tx: UnboundedSender<MobileTriggerEvent>) {
    loop {
        if let Err(e) = listen_once(&server_url, &jwt, tx.clone()).await {
            eprintln!("📱 Mobile sync: trigger listener error: {e}");
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn listen_once(
    server_url: &str,
    jwt: &str,
    tx: UnboundedSender<MobileTriggerEvent>,
) -> Result<()> {
    let ws_url = normalize_ws_url(server_url);
    let mut request = ws_url
        .as_str()
        .into_client_request()
        .context("Failed to create WebSocket request for mobile trigger")?;
    request
        .headers_mut()
        .insert(AUTHORIZATION, format!("Bearer {}", jwt).parse()?);

    let (ws, _) = connect_async(request)
        .await
        .context("Failed to connect to sync server for mobile trigger")?;
    let (mut write, mut read) = ws.split();

    let request_list = lst_proto::ClientMessage::RequestDocumentList;
    write
        .send(Message::Text(
            serde_json::to_string(&request_list)
                .context("Failed to serialize RequestDocumentList for mobile trigger")?,
        ))
        .await
        .context("Failed to send RequestDocumentList from mobile trigger")?;
    let _ = tx.send(MobileTriggerEvent::RemoteChange);

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(server_msg) = serde_json::from_str::<lst_proto::ServerMessage>(&text) {
                    match server_msg {
                        lst_proto::ServerMessage::NewChanges { .. }
                        | lst_proto::ServerMessage::DocumentList { .. }
                        | lst_proto::ServerMessage::Snapshot { .. } => {
                            let _ = tx.send(MobileTriggerEvent::RemoteChange);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("📱 Mobile sync: trigger WebSocket error: {e}");
                break;
            }
        }
    }

    Ok(())
}
