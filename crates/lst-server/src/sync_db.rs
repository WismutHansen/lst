use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::PathBuf;
use lst_proto::DocumentInfo;
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct SyncDb {
    pool: SqlitePool,
}

impl SyncDb {
    pub async fn new(mut base_dir: PathBuf) -> Result<Self> {
        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir)?;
        }
        base_dir.push("content.db");
        let db_url = format!("sqlite://{}", base_dir.to_string_lossy());
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS documents (
                doc_id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                encrypted_snapshot BLOB NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS document_changes (
                change_id INTEGER PRIMARY KEY AUTOINCREMENT,
                doc_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                encrypted_change BLOB NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(&pool)
        .await?;
        Ok(SyncDb { pool })
    }

    pub async fn list_documents(&self, user_id: &str) -> Result<Vec<DocumentInfo>> {
        let rows = sqlx::query(
            "SELECT doc_id, updated_at FROM documents WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| DocumentInfo {
                doc_id: row.get("doc_id"),
                updated_at: row.get::<DateTime<Utc>, _>("updated_at"),
            })
            .collect())
    }

    pub async fn get_snapshot(&self, doc_id: &str) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query("SELECT encrypted_snapshot FROM documents WHERE doc_id = ?")
            .bind(doc_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get("encrypted_snapshot")))
    }

    pub async fn save_snapshot(
        &self,
        doc_id: &str,
        user_id: &str,
        snapshot: &[u8],
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO documents (doc_id, user_id, encrypted_snapshot)
               VALUES (?, ?, ?)
               ON CONFLICT(doc_id) DO UPDATE SET
                   encrypted_snapshot = excluded.encrypted_snapshot,
                   updated_at = CURRENT_TIMESTAMP"#,
        )
        .bind(doc_id)
        .bind(user_id)
        .bind(snapshot)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn add_changes(
        &self,
        doc_id: &str,
        device_id: &str,
        changes: &[Vec<u8>],
    ) -> Result<()> {
        for c in changes {
            sqlx::query(
                "INSERT INTO document_changes (doc_id, device_id, encrypted_change) VALUES (?, ?, ?)",
            )
            .bind(doc_id)
            .bind(device_id)
            .bind(c)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}

