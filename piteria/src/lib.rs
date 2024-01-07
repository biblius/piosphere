use db::PiteriaDatabase;
use deployment::{nginx::NginxConfig, systemd::SystemdConfig};
use error::PiteriaError;
use socket::{Hello, PiteriaRequest, PiteriaTag, PiteriaWrite, ViewDeployment};
use std::process::{Command, Stdio};
use tokio::net::UnixStream;

pub mod db;
pub mod deployment;
pub mod error;
pub mod socket;

pub type PiteriaResult<T> = Result<T, PiteriaError>;

/// Default location for the DB file.
pub const PITERIA_DB_FILE: &str = "/opt/piteria/piteria.db";

/// Default location for the unix socket.
pub const PITERIA_SOCKET: &str = "/tmp/piteria";

/// Default location for the vhost file.
pub const NGINX_FILE_PATH: &str = "dump/hello.vhost"; // TODO: /etc/nginx/sites-enabled

/// Default location for the service file.
pub const SYSD_FILE_PATH: &str = "dump/hello.service"; // TODO: /etc/systemd/system/multi-target.user.wants

#[derive(Debug)]
pub struct PiteriaService {
    db: PiteriaDatabase,
}

impl PiteriaService {
    pub fn new(db: PiteriaDatabase) -> Self {
        Self { db }
    }

    pub async fn respond(&self, stream: &mut UnixStream, msg: PiteriaRequest) -> PiteriaResult<()> {
        match msg.tag {
            PiteriaTag::Hello => stream.write(Hello).await?,
            PiteriaTag::Overview => {
                let deployments = self.db.list_deployments().await?;
                stream.write(deployments).await?
            }
            PiteriaTag::ViewDeployment => {
                let id: ViewDeployment = bincode::deserialize(&msg.message)?;
                let deployment = self.view_deployment(id.0).await?;
                stream.write(deployment).await?
            }
        }

        Ok(())
    }

    async fn view_deployment(&self, id: i64) -> PiteriaResult<deployment::Deployment> {
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

    fn read_nginx_config(path: &str) -> PiteriaResult<NginxConfig> {
        let file = std::fs::read_to_string(path)?;
        NginxConfig::parse(&file)
    }

    fn read_sysd_config(path: &str) -> PiteriaResult<SystemdConfig> {
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
