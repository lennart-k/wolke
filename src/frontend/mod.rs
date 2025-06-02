use std::{fs::DirEntry, sync::Arc};

use actix_session::{
    SessionMiddleware,
    config::CookieContentSecurity,
    storage::{CookieSessionStore, SessionStore},
};
use actix_web::{
    Responder,
    cookie::{Key, SameSite},
    web::{self, Data, Path},
};
use askama::Template;
use askama_web::WebTemplate;
use serde::Deserialize;

use crate::filesystem::{Error, Filesystem, FilesystemProvider};

#[derive(Debug, Deserialize)]
struct PathComponents {
    pub mount: String,
    pub path: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "pages/browse.html")]
struct BrowseView {
    entries: Vec<DirEntry>,
}

async fn route_browse<FSP: FilesystemProvider>(
    path: Path<PathComponents>,
    fs_provider: Data<FSP>,
) -> Result<impl Responder, Error> {
    let PathComponents { mount, path } = path.into_inner();
    let path = path.unwrap_or_default();
    let fs = fs_provider.get_filesystem(&mount).await?;
    let entries = fs
        .list_dir(&path)
        .await?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BrowseView { entries })
}

#[derive(Debug, Clone, Deserialize)]
pub struct FrontendConfig {
    #[serde(serialize_with = "hex::serde::serialize")]
    #[serde(deserialize_with = "hex::serde::deserialize")]
    pub secret_key: [u8; 64],
}

pub fn session_middleware(frontend_secret: [u8; 64]) -> SessionMiddleware<impl SessionStore> {
    SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&frontend_secret))
        .cookie_secure(true)
        .cookie_same_site(SameSite::Strict)
        .cookie_content_security(CookieContentSecurity::Private)
        .build()
}

pub fn configure_frontend<FSP: FilesystemProvider>(
    cfg: &mut web::ServiceConfig,
    frontend_config: FrontendConfig,
    fs_provider: Arc<FSP>,
) {
    let scope = web::scope("")
        .wrap(session_middleware(frontend_config.secret_key))
        .app_data(Data::new(frontend_config.clone()))
        .app_data(Data::from(fs_provider))
        .service(
            web::scope("/mount/{mount}")
                .route("", web::get().to(route_browse::<FSP>))
                .route("/{path:.+}", web::get().to(route_browse::<FSP>)),
        );

    cfg.service(scope);
}
