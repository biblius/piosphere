use db::PiosphereDatabase;
use deployment::{nginx::NginxConfig, systemd::SystemdConfig};
use error::PiosphereError;
use socket::{
    message::{Hello, Overview, ViewDeployment},
    Message, PiosphereRequest, PiosphereTag, PiosphereWrite,
};
use std::process::{Command, Stdio};
use tokio::net::UnixStream;

pub mod db;
pub mod deployment;
pub mod error;
pub mod socket;

pub type PiosphereResult<T> = Result<T, PiosphereError>;

/// Default location for the DB file.
pub const PITERIA_DB_FILE: &str = "/opt/piosphere/piosphere.db";

/// Default location for the unix socket.
pub const PITERIA_SOCKET: &str = "/tmp/piosphere";

/// Default location for the vhost file.
pub const NGINX_FILE_PATH: &str = "dump/hello.vhost"; // TODO: /etc/nginx/sites-enabled

/// Default location for the service file.
pub const SYSD_FILE_PATH: &str = "dump/hello.service"; // TODO: /etc/systemd/system/multi-target.user.wants

#[derive(Debug)]
pub struct PiosphereService {
    db: PiosphereDatabase,
}

#[allow(async_fn_in_trait)]
pub trait Handler<M: Message> {
    async fn handle(&self, request: M) -> PiosphereResult<M::Response>;
}

pub struct PiosphereHandler;

impl Handler<Hello> for PiosphereService {
    async fn handle(&self, _: Hello) -> PiosphereResult<<Hello as Message>::Response> {
        Ok(Hello)
    }
}

impl Handler<Overview> for PiosphereService {
    async fn handle(&self, _: Overview) -> PiosphereResult<<Overview as Message>::Response> {
        self.db
            .list_deployments()
            .await
            .map_err(PiosphereError::from)
    }
}

impl Handler<ViewDeployment> for PiosphereService {
    async fn handle(
        &self,
        ViewDeployment(id): ViewDeployment,
    ) -> PiosphereResult<<ViewDeployment as Message>::Response> {
        self.view_deployment(&id).await
    }
}

impl PiosphereService {
    pub fn new(db: PiosphereDatabase) -> Self {
        Self { db }
    }

    pub async fn respond(
        &self,
        stream: &mut UnixStream,
        msg: PiosphereRequest,
    ) -> PiosphereResult<()> {
        handle! {self, stream, msg,
            Hello => Hello,
            Overview => Overview,
            ViewDeployment => ViewDeployment,
        }

        Ok(())
    }

    pub async fn view_deployment(&self, id: &str) -> PiosphereResult<deployment::Deployment> {
        let (deployment, nginx_cfg, sysd_cfg) = self.db.get_deployment(id).await?;

        let nginx_cfg = Self::read_nginx_config(&nginx_cfg.file_path)?;
        let sysd_cfg = Self::read_sysd_config(&sysd_cfg.file_path)?;

        Ok(deployment::Deployment::new(
            &deployment.name,
            &deployment.description,
            nginx_cfg,
            sysd_cfg,
        ))
    }

    fn read_nginx_config(path: &str) -> PiosphereResult<NginxConfig> {
        let file = std::fs::read_to_string(path)?;
        NginxConfig::parse(&file)
    }

    fn read_sysd_config(path: &str) -> PiosphereResult<SystemdConfig> {
        let file = std::fs::read_to_string(path)?;
        Ok(SystemdConfig::parse(&file))
    }
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
