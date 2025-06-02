use crate::{
    dav::{
        Error,
        fs::{FSResourceService, FSResourceServicePath},
    },
    filesystem::{Filesystem, FilesystemProvider},
};
use actix_web::{
    HttpResponse,
    http::StatusCode,
    web::{Data, Path, Payload},
};
use futures::StreamExt;
use std::io::Write;

pub async fn route_put<FSP: FilesystemProvider>(
    path: Path<FSResourceServicePath>,
    resource_service: Data<FSResourceService<FSP>>,
    mut payload: Payload,
) -> Result<HttpResponse, Error> {
    // TODO: Overwrite
    let filesystem = resource_service.0.get_filesystem(&path.mount).await?;
    let mut file = filesystem.create_file(&path.path).await?;
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
    }

    Ok(HttpResponse::build(StatusCode::CREATED).finish())
}
