use crate::dav::fs::FSPrincipalUri;
use crate::frontend::frontend_router;
use anyhow::Result;
use axum::extract::Request;
use axum::response::Response;
use axum::{Extension, Router, ServiceExt};
use clap::Parser;
use config::Config;
use figment::Figment;
use figment::providers::{Env, Format, Toml};
use filesystem::SimpleFilesystemProvider;
use headers::{HeaderMapExt, UserAgent};
use http::StatusCode;
use rustical_dav::resource::ResourceService;
use setup_tracing::setup_tracing;
use std::sync::Arc;
use std::time::Duration;
use tower::Layer;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::trace::TraceLayer;
use tracing::Span;
use tracing::field::display;

mod config;
mod dav;
mod filesystem;
mod frontend;
mod setup_tracing;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, env, default_value = "/etc/wolke/config.toml")]
    config_file: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config: Config = Figment::new()
        .merge(Toml::file(&args.config_file))
        .merge(Env::prefixed("WOLKE_").split("__"))
        .extract()?;

    setup_tracing(&config.tracing);

    let fs_provider = Arc::new(SimpleFilesystemProvider::new(config.fs.root_path));

    let app = Router::new()
        .with_state(())
        .route_service(
            "/dav/mount/{mount}",
            dav::fs::FSResourceService::new(fs_provider.clone()).axum_service(),
        )
        .route_service(
            "/dav/mount/{mount}/{*path}",
            dav::fs::FSResourceService::new(fs_provider).axum_service(),
        )
        .nest("/frontend", frontend_router())
        .layer(Extension(FSPrincipalUri))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request| {
                    tracing::info_span!(
                        "http-request",
                        status = tracing::field::Empty,
                        otel.name = tracing::field::display(format!(
                            "{} {}",
                            request.method(),
                            request.uri()
                        )),
                        ua = tracing::field::Empty,
                    )
                })
                .on_request(|req: &Request, span: &Span| {
                    span.record("method", display(req.method()));
                    span.record("path", display(req.uri()));
                    if let Some(ua) = req.headers().typed_get::<UserAgent>() {
                        span.record("ua", display(ua));
                    }
                })
                .on_response(|response: &Response, _latency: Duration, span: &Span| {
                    span.record("status", display(response.status()));
                    if response.status().is_server_error() {
                        tracing::error!("server error");
                    } else if response.status().is_client_error() {
                        match response.status() {
                            StatusCode::UNAUTHORIZED => {
                                // The iOS client always tries an unauthenticated request first so
                                // logging 401's as errors would clog up our logs
                                tracing::debug!("unauthorized");
                            }
                            StatusCode::NOT_FOUND => {
                                tracing::warn!("client error");
                            }
                            _ => {
                                tracing::error!("client error");
                            }
                        }
                    };
                })
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        tracing::error!("something went wrong")
                    },
                ),
        );

    let app = ServiceExt::<Request>::into_make_service(
        NormalizePathLayer::trim_trailing_slash().layer(app),
    );

    let listener =
        tokio::net::TcpListener::bind(&format!("{}:{}", config.http.host, config.http.port))
            .await?;

    axum::serve(listener, app).await?;
    Ok(())
}
