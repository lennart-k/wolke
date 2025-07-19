use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_owned(),
            port: 5000,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub http: HttpConfig,
    #[serde(default)]
    pub tracing: TracingConfig,

    pub fs: FSConfig,
}

#[derive(Deserialize, Serialize)]
pub struct FSConfig {
    pub root_path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct TracingConfig {
    pub opentelemetry: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            opentelemetry: true,
        }
    }
}
