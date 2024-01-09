use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

#[derive(Debug, Deserialize, Serialize)]
pub struct Deployment {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub id: String,
    pub deployment_id: i64,
    pub file_path: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug)]
pub struct PiosphereDatabase {
    client: SqlitePool,
}

impl PiosphereDatabase {
    /// Establish a connection pool at the specified sqlite file
    pub async fn new(file: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::new()
            .filename(file)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await?;
        Ok(Self { client: pool })
    }

    pub async fn migrate(&self) -> Result<(), sqlx::migrate::MigrateError> {
        sqlx::migrate!().run(&self.client).await
    }

    /// (Deploymeny, NginxConfig, SysdConfig)
    pub async fn get_deployment(&self, id: &str) -> sqlx::Result<(Deployment, Config, Config)> {
        let deployment = sqlx::query_as!(Deployment, "SELECT * FROM deployments WHERE id=?", id)
            .fetch_one(&self.client)
            .await?;

        let nginx_cfg = sqlx::query_as!(
            Config,
            "SELECT * FROM nginx_configs WHERE deployment_id=?",
            deployment.id,
        )
        .fetch_one(&self.client)
        .await?;

        let sysd_cfg = sqlx::query_as!(
            Config,
            "SELECT * FROM sysd_configs WHERE deployment_id=?",
            deployment.id,
        )
        .fetch_one(&self.client)
        .await?;

        Ok((deployment, nginx_cfg, sysd_cfg))
    }

    pub async fn insert_deployment(
        &self,
        deployment: &crate::deployment::Deployment,
    ) -> sqlx::Result<Deployment> {
        let mut tx = self.client.begin().await?;

        match {
            let deployment_new = sqlx::query_as!(
                Deployment,
                "INSERT INTO deployments(id, name, description) VALUES (?, ?, ?) RETURNING *",
                deployment.id,
                deployment.name,
                deployment.description
            )
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO nginx_configs (deployment_id, file_path) VALUES (?, ?)",
                deployment_new.id,
                deployment.nginx_cfg.file_location
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO sysd_configs (deployment_id, file_path) VALUES (?, ?)",
                deployment_new.id,
                deployment.service_cfg.file_location
            )
            .execute(&mut *tx)
            .await?;

            Result::<Deployment, sqlx::Error>::Ok(deployment_new)
        } {
            Ok(dep) => {
                tx.commit().await?;
                Ok(dep)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e)
            }
        }
    }

    pub async fn list_deployments(&self) -> sqlx::Result<Vec<Deployment>> {
        sqlx::query_as!(Deployment, "SELECT * FROM deployments")
            .fetch_all(&self.client)
            .await
    }

    pub async fn delete_deployment(&self, id: i64) -> sqlx::Result<u64> {
        let result = sqlx::query!("DELETE FROM deployments WHERE id=?", id)
            .execute(&self.client)
            .await;

        result.map(|res| res.rows_affected())
    }
}
