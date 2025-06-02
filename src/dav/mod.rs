mod error;
pub mod fs;
// pub mod principal;

use std::{
    convert::Infallible,
    future::{Ready, ready},
};

use actix_web::FromRequest;
pub use error::Error;
use rustical_dav::Principal;

#[derive(Debug, derive_more::From, Clone)]
pub struct User(pub String);

impl Principal for User {
    fn get_id(&self) -> &str {
        &self.0
    }
}

impl FromRequest for User {
    type Error = Infallible;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        _req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        ready(Ok(User("user".to_owned())))
    }
}
