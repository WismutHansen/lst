use anyhow::{Context, Result};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    receiver: mpsc::UnboundedReceiver<notify::Result<Event>>,
    last_emit: Instant,
}

impl FileWatcher {
    pub fn new(content_dir: &Path) -> Result<Self> {
        let (tx, receiver) = mpsc::unbounded_channel();
        
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(_) = tx.send(res) {
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
            last_emit: Instant::now(),
        })
    }

    pub async fn next_event(&mut self) -> Option<Event> {
        match self.receiver.recv().await {
            Some(Ok(mut event)) => {
                // simple debounce: wait briefly and drain extra events
                tokio::time::sleep(Duration::from_millis(100)).await;
                while let Ok(e) = self.receiver.try_recv() {
                    if let Ok(ev) = e {
                        event.paths.extend(ev.paths);
                    }
                }
                if self.last_emit.elapsed() < Duration::from_millis(100) {
                    return None;
                }
                self.last_emit = Instant::now();
                match event.kind {
                    notify::EventKind::Create(_)
                    | notify::EventKind::Modify(_)
                    | notify::EventKind::Remove(_) => Some(event),
                    _ => None,
                }
            }
            Some(Err(e)) => {
                eprintln!("File watcher error: {}", e);
                None
            }
            None => None,
        }
    }
}