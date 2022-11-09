use std::{net::SocketAddr, path::Path};

use anyhow::Result;
use kdl::KdlDocument;
use tokio::fs::read_to_string;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub services: Vec<String>,
}

impl Config {
    pub async fn init(path: impl AsRef<Path>) -> Result<Self> {
        let config_str = read_to_string(path).await?;
        let doc: KdlDocument = config_str.parse()?;
        let r = Self {
            bind_addr: doc
                .get_arg("bind_addr")
                .map(|s| s.as_string().unwrap().parse().unwrap())
                .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8080))),
            services: doc
                .get_args("services")
                .into_iter()
                .map(|i| i.as_string().unwrap().to_owned())
                .collect(),
        };
        Ok(r)
    }
}
