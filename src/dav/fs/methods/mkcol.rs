use crate::{
    dav::{
        Error,
        fs::{FSResourceService, FSResourceServicePath},
    },
    filesystem::{Filesystem, FilesystemProvider},
};
use axum::{
    body::Body,
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use http::StatusCode;

pub async fn route_mkcol<FSP: FilesystemProvider>(
    State(resource_service): State<FSResourceService<FSP>>,
    Path(path): Path<FSResourceServicePath>,
) -> Result<Response<Body>, Error> {
    let filesystem = resource_service.0.get_filesystem(&path.mount).await?;
    filesystem.create_dir(&path.path).await?;

    Ok(StatusCode::CREATED.into_response())
}
