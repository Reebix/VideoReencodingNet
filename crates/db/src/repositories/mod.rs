mod files_repository;

use crate::repositories::files_repository::{FilesRepository, FilesRepositoryImpl};
use sqlx::{Error, Pool, Sqlite};
use std::sync::Arc;

pub struct Repositories {
    #[allow(dead_code)]
    pool: Pool<Sqlite>,
    pub files_repo: Arc<dyn FilesRepository>,
}

impl Repositories {
    pub async fn new(db_pool: Pool<Sqlite>) -> Result<Self, Error> {
        let files_repo = Arc::new(FilesRepositoryImpl::new(db_pool.clone()));

        Ok(Self {
            pool: db_pool,
            files_repo,
        })
    }
}
