use crate::models::db_files::{Files};
use async_trait::async_trait;
use sqlx::{Error, Pool, Sqlite};

#[async_trait]
pub trait FilesRepository: Send + Sync + 'static {
    async fn get_all(&self) -> Result<Vec<Files>, Error>;
    async fn contains_path(&self, path: &str) -> Result<bool, Error>;
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

}
