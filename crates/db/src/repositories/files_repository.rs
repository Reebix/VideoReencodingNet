use crate::models::db_files::Files;
use async_trait::async_trait;
use sqlx::{Error, Pool, Sqlite};

#[async_trait]
pub trait FilesRepository: Send + Sync + 'static {
    async fn get_all(&self) -> Result<Vec<Files>, Error>;
    async fn contains_path(&self, path: &str) -> Result<bool, Error>;
    async fn add_files(&self, files: Vec<Files>) -> Result<(), Error>;
    async fn get_files_with_not_matching_codec(&self, codec: &str) -> Result<Vec<Files>, Error>;
}

pub struct FilesRepositoryImpl {
    pool: Pool<Sqlite>,
}

impl FilesRepositoryImpl {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { pool: db }
    }
}

struct NumResult {
    num: i64,
}

#[async_trait]
impl FilesRepository for FilesRepositoryImpl {
    async fn get_all(&self) -> Result<Vec<Files>, Error> {
        let tests: Vec<Files> = sqlx::query_as!(
            Files,
            r#"
SELECT  path, codec
FROM files
"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(tests)
    }

    async fn contains_path(&self, path: &str) -> Result<bool, Error> {
        let result: Option<NumResult> = sqlx::query_as!(
            NumResult,
            r#"
SELECT 1 as num
FROM files
WHERE path = ?
LIMIT 1
"#,
            path
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.is_some())
    }

    async fn add_files(&self, files: Vec<Files>) -> Result<(), Error> {
        let transaction = self.pool.begin().await?;
        for file in files {
            sqlx::query!(
                r#"
INSERT INTO files (path, codec)
VALUES (?, ?)
                "#,
                file.path,
                file.codec
            )
            .execute(&self.pool)
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    async fn get_files_with_not_matching_codec(&self, codec: &str) -> Result<Vec<Files>, Error> {
        let codec = format!("%{}%", codec);
        let files: Vec<Files> = sqlx::query_as!(
            Files,
            r#"
SELECT  path, codec
FROM files
WHERE codec not like ?
            "#,
            codec
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(files)
    }
}
