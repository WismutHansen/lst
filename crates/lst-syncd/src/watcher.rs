use anyhow::{Context, Result};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::UnboundedReceiver<notify::Result<Event>>,
}

impl FileWatcher {
    pub fn new(content_dir: &Path) -> Result<Self> {
        eprintln!(
            "DEBUG: FileWatcher::new called with path: {}",
            content_dir.display()
        );
        let (tx, receiver) = mpsc::unbounded_channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if tx.send(res).is_err() {
                    // Channel closed, watcher is being dropped
                }
            },
            Config::default(),
        )
        .context("Failed to create file watcher")?;

        watcher
            .watch(content_dir, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch directory: {}", content_dir.display()))?;

        Ok(Self {
            _watcher: watcher,
            receiver,
        })
    }

    pub async fn next_event(&mut self) -> Option<Event> {
        match self.receiver.recv().await {
            Some(Ok(event)) => {
                // Filter out events we don't care about
                match event.kind {
                    notify::EventKind::Create(_)
                    | notify::EventKind::Modify(_)
                    | notify::EventKind::Remove(_) => Some(event),
                    _ => None,
                }
            }
            Some(Err(e)) => {
                eprintln!("File watcher error: {e}");
                None
            }
            None => None,
        }
    }
}
