use std::path::PathBuf;

use sqlx::{SqlitePool, migrate::MigrateDatabase};

use crate::error::{DaemonError, ExpectExt};

#[derive(Clone, Debug)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self, DaemonError> {
        let db_path = Self::database_path();

        if let Some(dir) = db_path.parent() // This should never be none
            && !dir.is_dir()
        {
            std::fs::create_dir_all(dir)?;
        }

        let url = format!("sqlite:{}", db_path.display());

        if !sqlx::Sqlite::database_exists(&url).await? {
            sqlx::Sqlite::create_database(&url).await?
        }

        let pool = SqlitePool::connect(&url).await?;

        sqlx::migrate!().run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn record_message(&self, chat_id: i64, message_id: i64) -> Result<(), DaemonError> {
        sqlx::query(
            r#"
            INSERT INTO telegram_messages (chat_id, message_id, sent_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(chat_id)
        .bind(message_id)
        .bind(jiff::Timestamp::now().as_second())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub fn database_path() -> PathBuf {
        let dir = if cfg!(debug_assertions) {
            dirs::data_dir()
                .responsible_expect("XDG_DATA_HOME is not set")
                .join("egress")
        } else {
            PathBuf::from("/var/lib/egress")
        };

        dir.join("egress.sqlite")
    }
}
