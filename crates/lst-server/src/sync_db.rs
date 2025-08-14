use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::PathBuf;
use lst_proto::DocumentInfo;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct SyncDb {
    pool: SqlitePool,
}

impl SyncDb {
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
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
            r#"CREATE TABLE IF NOT EXISTS document_permissions (
                doc_id TEXT NOT NULL,
                user_email TEXT NOT NULL,
                permission_type TEXT NOT NULL,
                granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (doc_id, user_email),
                FOREIGN KEY (doc_id) REFERENCES documents(doc_id) ON DELETE CASCADE
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

    pub async fn list_documents(&self, user_email: &str) -> Result<Vec<DocumentInfo>> {
        let rows = sqlx::query(
            r#"SELECT DISTINCT d.doc_id, d.updated_at 
               FROM documents d
               JOIN document_permissions p ON d.doc_id = p.doc_id
               WHERE p.user_email = ?"#,
        )
        .bind(&user_email.to_lowercase())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let doc_id_str: String = row.get("doc_id");
                DocumentInfo {
                    doc_id: Uuid::parse_str(&doc_id_str).expect("Invalid UUID in database"),
                    updated_at: row.get::<DateTime<Utc>, _>("updated_at"),
                }
            })
            .collect())
    }

    pub async fn get_snapshot(&self, doc_id: &Uuid) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query("SELECT encrypted_snapshot FROM documents WHERE doc_id = ?")
            .bind(doc_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.get("encrypted_snapshot")))
    }

    pub async fn save_snapshot(
        &self,
        doc_id: &Uuid,
        user_id: &str,
        snapshot: &[u8],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        
        sqlx::query(
            r#"INSERT INTO documents (doc_id, user_id, encrypted_snapshot)
               VALUES (?, ?, ?)
               ON CONFLICT(doc_id) DO UPDATE SET
                   encrypted_snapshot = excluded.encrypted_snapshot,
                   updated_at = CURRENT_TIMESTAMP"#,
        )
        .bind(doc_id.to_string())
        .bind(&user_id.to_lowercase())
        .bind(snapshot)
        .execute(&mut *tx)
        .await?;
        
        sqlx::query(
            r#"INSERT OR IGNORE INTO document_permissions (doc_id, user_email, permission_type)
               VALUES (?, ?, 'owner')"#,
        )
        .bind(doc_id.to_string())
        .bind(&user_id.to_lowercase())
        .execute(&mut *tx)
        .await?;
        
        tx.commit().await?;
        Ok(())
    }

    pub async fn add_changes(
        &self,
        doc_id: &Uuid,
        device_id: &str,
        changes: &[Vec<u8>],
    ) -> Result<()> {
        for c in changes {
            sqlx::query(
                "INSERT INTO document_changes (doc_id, device_id, encrypted_change) VALUES (?, ?, ?)",
            )
            .bind(doc_id.to_string())
            .bind(device_id)
            .bind(c)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    
}

