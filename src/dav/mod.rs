mod error;
pub mod fs;
use axum::extract::FromRequestParts;
pub use error::Error;
use rustical_dav::Principal;
use std::convert::Infallible;

#[derive(Debug, derive_more::From, Clone)]
pub struct User(pub String);

impl Principal for User {
    fn get_id(&self) -> &str {
        &self.0
    }
}

impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(User("user".to_owned()))
    }
}
