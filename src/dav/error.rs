use axum::{body::Body, response::Response};
use http::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Dav(#[from] rustical_dav::Error),

    #[error(transparent)]
    XmlDecode(#[from] rustical_xml::XmlError),

    #[error(transparent)]
    FS(#[from] crate::filesystem::Error),

    #[error(transparent)]
    Axum(#[from] axum::Error),
}

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::FS(err) => err.status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl axum::response::IntoResponse for Error {
    fn into_response(self) -> Response {
        Response::builder()
            .status(self.status_code())
            .body(Body::new(self.to_string()))
            .expect("This must work")
    }
}
