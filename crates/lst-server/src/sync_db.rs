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

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS provision_requests (
                provisioning_id TEXT PRIMARY KEY,
                public_key BLOB NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )"#,
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS provision_packages (
                provisioning_id TEXT PRIMARY KEY,
                encrypted_master_key BLOB NOT NULL,
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
        .bind(user_email)
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
        let mut tx = self.pool.begin().await?;
        
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
        .execute(&mut *tx)
        .await?;
        
        sqlx::query(
            r#"INSERT OR IGNORE INTO document_permissions (doc_id, user_email, permission_type)
               VALUES (?, ?, 'owner')"#,
        )
        .bind(doc_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        
        tx.commit().await?;
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

    pub async fn has_access(&self, doc_id: &str, user_email: &str, required_permission: &str) -> Result<bool> {
        let permissions = ["owner", "writer", "reader"];
        let required_level = permissions.iter().position(|&p| p == required_permission).unwrap_or(2);
        
        let row = sqlx::query(
            "SELECT permission_type FROM document_permissions WHERE doc_id = ? AND user_email = ?"
        )
        .bind(doc_id)
        .bind(user_email)
        .fetch_optional(&self.pool)
        .await?;
        
        match row {
            Some(row) => {
                let user_permission: String = row.get("permission_type");
                let user_level = permissions.iter().position(|&p| p == user_permission).unwrap_or(2);
                Ok(user_level <= required_level)
            }
            None => Ok(false)
        }
    }

    pub async fn grant_access(&self, doc_id: &str, user_email: &str, permission_type: &str) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO document_permissions (doc_id, user_email, permission_type)
               VALUES (?, ?, ?)
               ON CONFLICT(doc_id, user_email) DO UPDATE SET
                   permission_type = excluded.permission_type,
                   granted_at = CURRENT_TIMESTAMP"#,
        )
        .bind(doc_id)
        .bind(user_email)
        .bind(permission_type)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn revoke_access(&self, doc_id: &str, user_email: &str) -> Result<()> {
        sqlx::query(
            "DELETE FROM document_permissions WHERE doc_id = ? AND user_email = ? AND permission_type != 'owner'"
        )
        .bind(doc_id)
        .bind(user_email)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn create_provision_request(&self, provisioning_id: &str, public_key: &[u8]) -> Result<()> {
        sqlx::query(
            "INSERT INTO provision_requests (provisioning_id, public_key) VALUES (?, ?)"
        )
        .bind(provisioning_id)
        .bind(public_key)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_provision_package(&self, provisioning_id: &str, package: &[u8]) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO provision_packages (provisioning_id, encrypted_master_key) VALUES (?, ?)"
        )
        .bind(provisioning_id)
        .bind(package)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_provision_package(&self, provisioning_id: &str) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query(
            "SELECT encrypted_master_key FROM provision_packages WHERE provisioning_id = ?"
        )
        .bind(provisioning_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.get("encrypted_master_key")))
    }
}

