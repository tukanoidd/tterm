use std::path::PathBuf;

use derive_more::Deref;
use directories::ProjectDirs;
use rootcause::Result;
use surql_macros::surql_check;
use surrealdb::{
    Surreal,
    engine::local::Db,
    types::{RecordId, SurrealValue},
};

#[derive(Deref)]
pub struct DbState {
    db: Surreal<Db>,
}

impl DbState {
    pub async fn new(project_dirs: ProjectDirs) -> Result<Self> {
        let data_dir = project_dirs.data_local_dir();

        if !data_dir.exists() {
            tokio::fs::create_dir_all(data_dir).await?;
        }

        let db_path = data_dir.join("db");

        let db = Surreal::new(db_path).await?;

        db.use_ns("tterm").use_db("data").await?;

        let db = Self::init_v1(db).await?;

        let res = Self { db };

        Ok(res)
    }

    async fn init_v1(db: Surreal<Db>) -> Result<Surreal<Db>> {
        let transaction = db.begin().await?;

        transaction
            // directories
            .query(surql_check!(
                "
                DEFINE TABLE IF NOT EXISTS directories
                    SCHEMAFULL
                    COMMENT 'table of indexed directories';

                DEFINE FIELD IF NOT EXISTS path ON TABLE directories
                    TYPE string;

                DEFINE INDEX directory_path ON TABLE directories COLUMNS path UNIQUE;
            "
            ))
            // files
            .query(surql_check!(
                "
                DEFINE TABLE IF NOT EXISTS files
                    SCHEMAFULL
                    COMMENT 'table of indexed files';

                DEFINE FIELD IF NOT EXISTS path ON TABLE files
                    TYPE string;

                DEFINE INDEX file_path ON TABLE files COLUMNS path UNIQUE;
            "
            ))
            // symlinks
            .query(surql_check!(
                "
                DEFINE TABLE IF NOT EXISTS symlinks
                    SCHEMAFULL
                    COMMENT 'table of indexed symlinks';

                DEFINE FIELD IF NOT EXISTS path ON TABLE symlinks
                    TYPE string;

                DEFINE INDEX symlink_path ON TABLE symlinks COLUMNS path UNIQUE;
            "
            ))
            .query(surql_check!(
                "
                DEFINE TABLE IF NOT EXISTS symlink_to
                    TYPE RELATION FROM symlinks TO directories|files|symlinks
                    COMMENT 'relation table of which nodes symlinks point to'
            "
            ))
            // util
            .query(surql_check!(
                "
                DEFINE TABLE IF NOT EXISTS parent_directory
                    TYPE RELATION FROM directories TO directories|files|symlink ENFORCED
                    COMMENT 'relation table of parent directories for indexed nodes'
            "
            ))
            .await?;

        let db = transaction.commit().await?;

        Ok(db)
    }
}

#[derive(Debug, Clone)]
pub enum DbAction {
    IndexPath(PathBuf),
}

#[derive(SurrealValue)]
pub struct DbFile {
    pub id: RecordId,
    pub path: String,
}
