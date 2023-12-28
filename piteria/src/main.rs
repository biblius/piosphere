use crate::{
    db::db_pool,
    socket::client::Client,
    systemd::{SysdInstallConfig, SysdServiceConfig, SysdUnitConfig, SystemdConfig},
};
use db::PiteriaDatabase;
use error::PiteriaError;
use nginx::{parse_vhost_file, NginxConfig, NginxLocation};
use serde::{Deserialize, Serialize};
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use socket::{server::start_server, PiteriaMessage};
use sqlx::SqlitePool;
use std::{
    fs,
    io::stdin,
    process::{Command, Stdio},
};

pub mod db;
pub mod error;
pub mod nginx;
pub mod socket;
pub mod systemd;

pub type PiteriaResult<T> = Result<T, PiteriaError>;

pub const PITERIA_DB_FILE: &str = "/opt/piteria/piteria.db";

#[tokio::main]
async fn main() {
    let mut signals = Signals::new([SIGTERM, SIGINT]).unwrap();

    let var = std::env::args().collect::<Vec<_>>();

    let pool = db_pool("piteria.db")
        .await
        .expect("Could not establish DB pool");

    demo(&pool).await;

    let db = PiteriaDatabase::new(pool);
    dbg!(db.list_deployments().await);
    //if var.get(1).is_some() {
    //start_server("/tmp/sock")
    //.await
    //.expect("Error closing server");
    //} else {
    //println!("Starting client");
    //let client = Client::new("/tmp/sock")
    //.await
    //.expect("Could not connect to Piteria server");

    //let mut buf = String::new();
    //stdin().read_line(&mut buf).unwrap();
    //let res = client
    //.request(PiteriaMessage::Hello)
    //.await
    //.expect("error in request");
    //println!("Got response: {:?}", res);

    //let signals = tokio::spawn(async move {
    //for sig in signals.forever() {
    //println!("Received signal {:?}", sig);

    //if sig == SIGINT {
    //let result = client.close().await;
    //return result;
    //}
    //}
    //unreachable!()
    //});

    //let _ = signals.await.expect("error while shutting down");
    //}
}

async fn process_message(message: PiteriaMessage) -> PiteriaMessage {
    println!("Processing {:?}", message);
    PiteriaMessage::Hello
}

async fn demo(pool: &SqlitePool) {
    let systemd_cfg = SystemdConfig {
        file_location: "dump/hello2.service".to_string(),
        unit: SysdUnitConfig::default(),
        service: SysdServiceConfig::default(),
        install: SysdInstallConfig::default(),
    };

    let nginx_cfg = NginxConfig {
        file_location: "dump/hello.vhost".to_string(),
        listen: 42069,
        access_log: None,
        server_name: "mysite.com".to_string(),
        location: vec![NginxLocation::new(), NginxLocation::new()],
    };

    println!("Running migrations");
    sqlx::migrate!().run(pool).await.unwrap();

    println!("Migrations ran");

    sqlx::query!("INSERT INTO deployments(name, description) VALUES ('foo','bar')")
        .execute(pool)
        .await
        .unwrap();

    sqlx::query!("DELETE FROM deployments WHERE name = 'foo'")
        .execute(pool)
        .await
        .unwrap();

    setup_deployment_files(nginx_cfg, systemd_cfg);
    let vhost = fs::read_to_string("dump/hello.vhost").unwrap();

    dbg!(parse_vhost_file(&vhost).unwrap());
}

pub fn invoke_sysd() {
    let res = Command::new("systemctl")
        .arg("show")
        .arg("postgres")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    println!(
        "{}",
        String::from_utf8(res.wait_with_output().unwrap().stdout).unwrap(),
    );
}

pub fn setup_deployment_files(nginx_cfg: NginxConfig, systemd_cfg: SystemdConfig) {
    setup_nginx_file(nginx_cfg);
    setup_systemd_service(systemd_cfg);
}

fn setup_nginx_file(config: NginxConfig) {
    let path = &config.file_location;
    fs::write(path, config.to_string()).unwrap()
}

fn setup_systemd_service(config: SystemdConfig) {
    let path = &config.file_location;
    fs::write(path, config.to_string()).unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Deployment {
    /// Deployment ID in Sqlite.
    id: i64,

    /// User defined name of the deployment.
    pub name: String,

    /// User defined description, not to be confused with the systemd description.
    /// If a systemd description is not defined, this one is used for it.
    pub description: String,

    /// The systemd service file.
    pub service_cfg: SystemdConfig,

    /// The nginx vhost file.
    pub nginx_cfg: NginxConfig,
}
