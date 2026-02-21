use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

pub fn frontend_router() -> Router {
    Router::new().fallback_service(
        ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/frontend/dist")).fallback(
            ServeFile::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/frontend/dist/index.html"
            )),
        ),
    )
}
