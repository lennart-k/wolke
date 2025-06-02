use actix_web::HttpServer;
use anyhow::Result;
use app::make_app;
use clap::Parser;
use config::Config;
use figment::Figment;
use figment::providers::{Env, Format, Toml};
use setup_tracing::setup_tracing;

mod app;
mod config;
mod dav;
mod filesystem;
mod frontend;
mod setup_tracing;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, env, default_value = "/etc/file-server/config.toml")]
    config_file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config: Config = Figment::new()
        // TODO: What to do when config file does not exist?
        .merge(Toml::file(&args.config_file))
        .merge(Env::prefixed("FS_").split("__"))
        .extract()?;

    setup_tracing(&config.tracing);

    HttpServer::new(move || make_app("./public/".to_owned()))
        .bind((config.http.host, config.http.port))?
        // Workaround for a weird bug where
        // new requests might timeout since they cannot properly reuse the connection
        // https://github.com/lennart-k/rustical/issues/10
        // .keep_alive(KeepAlive::Disabled)
        .run()
        .await?;
    Ok(())
}
