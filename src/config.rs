use std::path::Path;

use anyhow::Result;
use kdl::KdlDocument;
use tokio::fs::read_to_string;

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_port: u16,
    pub services: Vec<String>,
}

impl Config {
    pub async fn init(path: impl AsRef<Path>) -> Result<Self> {
        let config_str = read_to_string(path).await?;
        let doc: KdlDocument = config_str.parse()?;
        let r = Self {
            listen_port: doc
                .get_arg("listen_port")
                .map(|i| i.as_i64().unwrap() as u16)
                .unwrap_or(8080),
            services: doc
                .get_args("services")
                .into_iter()
                .map(|i| i.as_string().unwrap().to_owned())
                .collect(),
        };
        Ok(r)
    }
}
