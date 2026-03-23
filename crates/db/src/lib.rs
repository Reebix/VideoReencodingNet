use sqlx::{Pool, Sqlite, SqlitePool};
use sqlx::migrate::MigrateDatabase;
use tracing::{error, info};

pub mod models;
pub mod repositories;

const DB_URL: &str = "sqlite://sqlite.db";


pub async fn init_db() -> Result<Pool<Sqlite>, anyhow::Error> {
    info!("db init");

    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => info!("Create db success"),
            Err(error) => error!("error: {}", error),
        }
    } else {
        info!("Database already exists");
    }

    let pool = SqlitePool::connect(DB_URL).await.unwrap();

    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let migrations = std::path::Path::new(&crate_dir).join("../../migrations");


    info!("migrations: {}", migrations.display());

    let migration_results = sqlx::migrate::Migrator::new(migrations)
        .await
        .unwrap()
        .run(&pool)
        .await;

    info!("migration: {:?}", migration_results);

    match migration_results {
        Ok(_) => info!("Migration success"),
        Err(error) => {
            error!("error: {}", error);
        }
    }

    Ok(pool)
}
