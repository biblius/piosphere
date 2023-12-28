use chrono::NaiveDateTime;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

#[derive(Debug)]
pub struct Deployment {
    id: i64,
    name: String,
    description: String,
    created_at: NaiveDateTime,
}

#[derive(Debug)]
pub struct CreateDeployment<'a> {
    name: &'a str,
    description: &'a str,
}

impl<'a> From<&'a crate::Deployment> for CreateDeployment<'a> {
    fn from(value: &crate::Deployment) -> Self {
        Self {
            name: &value.name,
            description: &value.description,
        }
    }
}

#[derive(Debug)]
pub struct CreateConfig<'a> {
    deployment_id: i64,
    file_path: &'a str,
}

#[derive(Debug)]
pub struct Config {
    id: i64,
    deployment_id: i64,
    file_path: String,
    created_at: NaiveDateTime,
}

/// Establish a connection pool at the specified sqlite file
pub async fn db_pool(file: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(file)
        .create_if_missing(true);

    SqlitePool::connect_with(options).await
}

pub struct PiteriaDatabase {
    client: SqlitePool,
}

impl PiteriaDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        Self { client: pool }
    }

    pub async fn insert_deployment(&self, deployment: &crate::Deployment) -> sqlx::Result<u64> {
        let deplyoment = CreateDeployment::from(deployment);
        self.client.begin().await?;
        // sqlx::query!("")
        todo!()
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
