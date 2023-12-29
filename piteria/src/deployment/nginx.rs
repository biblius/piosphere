use nom::{
    self,
    bytes::complete::{is_not, tag},
    character::complete::char,
    sequence::delimited,
    IResult,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::{PiteriaError, PiteriaResult, NGINX_FILE_PATH};

#[derive(Debug, Serialize, Deserialize)]
pub struct NginxConfig {
    /// Absolute path to the nginx config file.
    ///
    /// By default this should be in /etc/nginx/sites-enabled
    pub file_location: String,

    /// Sets the `listen` directive in the `server` to this value.
    ///
    /// 80 is the default.
    pub listen: usize,

    /// The public facing domain of the server. Used by Nginx
    /// for pattern matching and forwarding requests.
    ///
    /// Example: `mysite.org`
    pub server_name: String,

    /// Location of the application's access log
    pub access_log: Option<String>,

    /// Used by Nginx to determine where to forward the request, based on the url.
    /// For example, if the location path is set to `/location/` (note the trailing slash),
    /// all requests matching `mysite.org/location` will be forwarded to `proxy_pass`.
    pub location: Vec<NginxLocation>,
}

impl NginxConfig {
    pub fn parse(input: &str) -> Result<NginxConfig, PiteriaError> {
        let lines = input.lines();

        let mut config = NginxConfig::default();
        let mut location = NginxLocation::default();

        let mut in_server = false;
        let mut in_location = false;

        for line in lines {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            if line.starts_with("server") && line.ends_with('{') {
                in_server = true;
                continue;
            }

            if in_location && line == "}" {
                in_location = false;
                config.location.push(location);
                location = NginxLocation::default();
                continue;
            }

            if in_server && line == "}" {
                in_server = false;
                continue;
            }

            if in_location {
                if line.starts_with("proxy_pass ") {
                    let pass: IResult<&str, &str> =
                        delimited(tag("proxy_pass "), is_not(";"), char(';'))(line);
                    match pass {
                        Ok((_, pass)) => location.proxy_pass = pass.to_string(),
                        Err(_) => {
                            return Err(PiteriaError::NginxParse(format!(
                                "Invalid proxy_pass at: {line}"
                            )))
                        }
                    }
                } else {
                    let Some((key, value)) = line.split_once(' ') else {
                        return Err(PiteriaError::NginxParse(format!(
                            "Invalid location directive at: {line}"
                        )));
                    };
                    location
                        .directives
                        .push((key.to_string(), value.to_string()))
                }
                continue;
            }

            if in_server {
                if line.starts_with("location") {
                    in_location = true;
                    let loc = line.split(' ');
                    for loc in loc.skip(1).take_while(|el| *el != "{") {
                        location.paths.push(loc.to_string());
                    }
                    continue;
                }

                if line.ends_with(';') {
                    let Some((key, value)) = line.split_once(' ') else {
                        continue;
                    };

                    if value.is_empty() {
                        continue;
                    }

                    let value = &value[..value.len() - 1];

                    match key {
                        "listen" => {
                            config.listen = value.parse().map_err(|_| {
                                PiteriaError::NginxParse(format!(
                                    "Invalid `listen` port value: {value}"
                                ))
                            })?
                        }
                        "access_log" => config.access_log = Some(value.to_string()),
                        "server_name" => config.server_name = value.to_string(),
                        _ => {}
                    }
                }
            }
        }

        Ok(config)
    }

    pub fn write_to_file(&self) -> PiteriaResult<()> {
        let path = &self.file_location;
        std::fs::write(path, self.to_string()).map_err(PiteriaError::from)
    }
}

impl Default for NginxConfig {
    fn default() -> Self {
        Self {
            file_location: NGINX_FILE_PATH.to_string(),
            listen: 80,
            server_name: Default::default(),
            access_log: None,
            location: vec![],
        }
    }
}

impl Display for NginxConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let NginxConfig {
            server_name,
            listen,
            location,
            ..
        } = self;
        writeln!(f, "server {{\n  listen {listen};")?;
        writeln!(f, "  server_name {server_name};")?;
        for location in location {
            writeln!(f, "  {location}")?;
        }
        writeln!(f, "}}")
    }
}

/// Key value pairs for an Nginx location.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NginxLocation {
    /// Determines which paths will get forwarded to `proxy_pass`
    pub paths: Vec<String>,

    /// A list of Nginx directives inside a `location` block.
    pub directives: Vec<(String, String)>,

    /// The address where the app will be listening on.
    pub proxy_pass: String,
}

impl NginxLocation {
    pub fn new() -> Self {
        Self {
            paths: vec!["/".to_string()],
            directives: vec![
                ("proxy_set_header".to_string(), "Host $host".to_string()),
                (
                    "proxy_set_header".to_string(),
                    "Upgrade $http_upgrade".to_string(),
                ),
                (
                    "proxy_set_header".to_string(),
                    "Connection \"upgrade\"".to_string(),
                ),
                (
                    "proxy_set_header".to_string(),
                    "X-Real-Ip $remote_addr".to_string(),
                ),
                (
                    "proxy_set_header".to_string(),
                    "X-Forwarded-For $proxy_add_x_forwarded_for".to_string(),
                ),
                (
                    "proxy_set_header".to_string(),
                    "X-Scheme $scheme".to_string(),
                ),
            ],

            proxy_pass: "http://localhost:42069/".to_string(),
        }
    }
}

impl Display for NginxLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let NginxLocation {
            paths,
            directives,
            proxy_pass,
        } = self;
        write!(f, "location ")?;

        for path in paths {
            write!(f, "{path} ")?;
        }

        writeln!(f, "{{")?;

        writeln!(f, "    proxy_pass {proxy_pass};")?;

        for (directive, value) in directives {
            writeln!(f, "    {directive} {value};")?;
        }

        write!(f, "  }}")
    }
}
