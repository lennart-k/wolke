use crate::{
    dav::{
        Error,
        fs::{FSResourceService, FSResourceServicePath},
    },
    filesystem::{Filesystem, FilesystemProvider},
};
use actix_web::{
    HttpRequest, HttpResponse,
    dev::ResourceDef,
    web::{Data, Path},
};
use percent_encoding::percent_decode_str;

pub async fn route_move<FSP: FilesystemProvider>(
    path: Path<FSResourceServicePath>,
    resource_service: Data<FSResourceService<FSP>>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let destination =
        percent_decode_str(req.headers().get("Destination").unwrap().to_str().unwrap())
            .decode_utf8()
            .unwrap();
    // let destination = req.headers().get("Destination").unwrap().to_str().unwrap();
    let mut destination = actix_web::dev::Path::new(destination.as_ref());
    dbg!(&destination);

    assert!(
        ResourceDef::prefix(req.full_url().origin().unicode_serialization())
            .join(&ResourceDef::new("/mount/{mount}/{path:.+}"))
            .capture_match_info(&mut destination)
    );
    let dest_path: FSResourceServicePath = FSResourceServicePath {
        mount: destination.get("mount").unwrap().to_owned(),
        path: destination.get("path").unwrap().to_owned(),
    };
    assert_eq!(&path.mount, &dest_path.mount);
    // req.resource_map().match_pattern
    let filesystem = resource_service.0.get_filesystem(&path.mount).await?;
    dbg!(&dest_path.path);
    filesystem.mv(&path.path, &dest_path.path).await?;

    Ok(HttpResponse::Ok().finish())
}
