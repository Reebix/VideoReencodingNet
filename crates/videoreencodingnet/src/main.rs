use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();
    logging::init_logger()?;

    let config = config::init_config();

    let db_pool = db::init_db().await?;

    let repositories = db::repositories::Repositories::new(db_pool.clone()).await?;
    let repositories = Arc::new(repositories);

    let path = config.path.clone();
    let repo_clone = repositories.clone();
    tokio::spawn(async move {
        let path = std::path::Path::new(&path);
        info!("Beginning scan of {}", path.display());
        media::scan(path, &repo_clone).await;
        info!("Scan completed for {}", path.display());
        let files = repo_clone
            .files_repo
            .get_files_with_not_matching_codec("av1")
            .await
            .unwrap_or_default();
        info!("Files with not matching codec: {}", files.len());
    });

    let (listener, router) = api::server::init(repositories, &config).await?;

    info!("Server listening on http://0.0.0.0:{}", config.port);
    let axum_result = axum::serve(listener, router).await;

    Ok(())
}
