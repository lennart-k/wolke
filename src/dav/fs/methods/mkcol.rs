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

pub async fn route_mkcol<FSP: FilesystemProvider>(
    path: Path<FSResourceServicePath>,
    resource_service: Data<FSResourceService<FSP>>,
) -> Result<HttpResponse, Error> {
    let filesystem = resource_service.0.get_filesystem(&path.mount).await?;
    filesystem.create_dir(&path.path).await?;
    Ok(HttpResponse::build(StatusCode::CREATED).finish())
}
