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
use futures::StreamExt;
use http::{Request, StatusCode};
use std::io::Write;

pub async fn route_put<FSP: FilesystemProvider>(
    State(resource_service): State<FSResourceService<FSP>>,
    Path(path): Path<FSResourceServicePath>,
    req: Request<Body>,
) -> Result<Response<Body>, Error> {
    let mut stream = req.into_body().into_data_stream();

    let filesystem = resource_service.0.get_filesystem(&path.mount).await?;
    let mut file = filesystem.create_file(&path.path).await?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
    }

    Ok(StatusCode::CREATED.into_response())
}
