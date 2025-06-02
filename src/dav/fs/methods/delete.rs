use crate::{
    dav::{
        Error,
        fs::{FSResourceService, FSResourceServicePath},
    },
    filesystem::{Filesystem, FilesystemProvider},
};
use actix_web::{
    HttpResponse, Responder,
    web::{Data, Path},
};

pub async fn route_delete<FSP: FilesystemProvider>(
    path: Path<FSResourceServicePath>,
    resource_service: Data<FSResourceService<FSP>>,
) -> Result<impl Responder, Error> {
    let filesystem = resource_service.0.get_filesystem(&path.mount).await?;
    filesystem.delete_file(&path.path).await?;
    Ok(HttpResponse::Ok().finish())
}
