use std::fmt::Display;

#[derive(Debug)]
pub struct SystemdConfig {
    /// Absolute path to the systemd service file.
    pub file_location: String,

    /// https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#%5BUnit%5D%20Section%20Options
    pub unit: SysdUnitConfig,

    /// https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html#Options
    pub service: SysdServiceConfig,

    /// https://www.freedesktop.org/software/systemd/man/latest/systemd.unit.html#%5BInstall%5D%20Section%20Options
    pub install: SysdInstallConfig,
}

impl Display for SystemdConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.unit)?;
        writeln!(f, "{}", self.service)?;
        writeln!(f, "{}", self.install)
    }
}

#[derive(Debug)]
pub struct SysdUnitConfig {
    /// The unit description under \[Unit\]
    pub description: String,
}

impl Display for SysdUnitConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[Unit]")?;
        writeln!(f, "Description={}", self.description)
    }
}

#[derive(Debug)]
pub struct SysdInstallConfig {
    pub wanted_by: String,
}

impl Default for SysdInstallConfig {
    fn default() -> Self {
        Self {
            wanted_by: "multi-user.target".to_string(),
        }
    }
}

impl Display for SysdInstallConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[Install]")?;
        writeln!(f, "WantedBy={}", self.wanted_by)
    }
}
/// Configuration for systemd.
#[derive(Debug)]
pub struct SysdServiceConfig {
    /// The command used by systemd that starts the application.
    pub exec_start: String,

    /// Systemd restart configuration.
    pub restart: RestartOption,

    /// Which user systemd uses for the service.
    pub user: String,

    /// Which group systemd uses for the service.
    pub group: String,

    /// Environment variables for the application.
    pub env: Vec<(String, String)>,

    /// The working directory, i.e. the absolute path where
    /// other systemd commands such as `start_cmd` will be executed in.
    pub dir: String,
}

impl Display for SysdServiceConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let SysdServiceConfig {
            exec_start,
            restart,
            user,
            group,
            env,
            dir,
        } = self;
        writeln!(f, "[Service]")?;
        writeln!(f, "ExecStart={exec_start}")?;
        writeln!(f, "Restart={restart}")?;
        writeln!(f, "User={user}")?;
        writeln!(f, "Group={group}")?;
        for (key, value) in env {
            writeln!(f, "Environment={key}={value}")?;
        }
        writeln!(f, "WorkingDirectory={dir}")
    }
}

/// Systemd service restart options
#[derive(Debug)]
pub enum RestartOption {
    No,
    OnSuccess,
    OnFailure,
    OnAbnormal,
    OnWatchdog,
    OnAbort,
    Always,
}

impl Display for RestartOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestartOption::No => write!(f, "no"),
            RestartOption::OnSuccess => write!(f, "no"),
            RestartOption::OnFailure => write!(f, "on-failure"),
            RestartOption::OnAbnormal => write!(f, "on-abnormal"),
            RestartOption::OnWatchdog => write!(f, "on-watchdog"),
            RestartOption::OnAbort => write!(f, "on-abort"),
            RestartOption::Always => write!(f, "always"),
        }
    }
}
