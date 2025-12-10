use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::path::PathBuf;
use anyhow::Result;

mod migrations;
pub mod oauth_state;
pub mod providers;
pub mod sessions;
pub mod settings;
pub mod usage;
pub mod users;

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConn = PooledConnection<SqliteConnectionManager>;

#[derive(Clone)]
pub struct Database {
    pool: DbPool,
}

impl Database {
    pub fn new(database_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let manager = SqliteConnectionManager::file(&database_path);
        let pool = Pool::builder()
            .max_size(8)
            .build(manager)?;
        
        let db = Self { pool };
        migrations::run(&db)?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)?;
        
        let db = Self { pool };
        migrations::run(&db)?;
        Ok(db)
    }
    
    pub fn conn(&self) -> Result<DbConn> {
        Ok(self.pool.get()?)
    }
    
    pub fn with_conn<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&rusqlite::Connection) -> Result<T>,
    {
        let conn = self.pool.get()?;
        f(&conn)
    }
}

pub fn init() -> Result<Database> {
    let path = std::env::var("DATABASE_PATH")
        .unwrap_or_else(|_| "proxypal.db".to_string());
    let db_path = PathBuf::from(path);
    Database::new(db_path)
}
