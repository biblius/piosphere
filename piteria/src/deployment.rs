use serde::{Deserialize, Serialize};

use crate::PiteriaResult;

use self::{nginx::NginxConfig, systemd::SystemdConfig};

pub mod nginx;
pub mod systemd;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Deployment {
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

impl Deployment {
    pub fn new(name: &str, desc: &str, nginx: NginxConfig, sysd: SystemdConfig) -> Self {
        Self {
            name: name.to_string(),
            description: desc.to_string(),
            service_cfg: sysd,
            nginx_cfg: nginx,
        }
    }
    pub fn write_config(&self) -> PiteriaResult<()> {
        self.nginx_cfg.write_to_file()?;
        self.service_cfg.write_to_file()?;
        Ok(())
    }
}
