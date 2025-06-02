use crate::{
    dav::{
        Error,
        fs::{FSResourceService, FSResourceServicePath},
    },
    filesystem::{FileReader, Filesystem, FilesystemProvider},
};
use actix_files::HttpRange;
use actix_web::{
    HttpRequest, HttpResponse, Responder,
    body::SizedStream,
    http::{
        StatusCode,
        header::{
            self, Charset, ContentDisposition, DispositionParam, ExtendedValue, HeaderValue,
            HttpDate,
        },
    },
    web::{Data, Path},
};
use percent_encoding::{CONTROLS, percent_encode};
use rustical_dav::resource::ResourceService;
use std::os::unix::ffi::OsStrExt;

// A lot of code here is stolen from actix-files
// However, I'm not just using NamedFile since I want to be filesystem-agnostic
pub async fn route_get<FSP: FilesystemProvider>(
    req: HttpRequest,
    path: Path<FSResourceServicePath>,
    resource_service: Data<FSResourceService<FSP>>,
) -> Result<impl Responder, Error> {
    dbg!(&req);
    // TODO: Why does extracting zip files not work with gvfs?
    let resource = resource_service.get_resource(&path).await?;
    let filename = resource.path.file_name().unwrap();
    let filename = percent_encode(filename.as_bytes(), CONTROLS).to_string();
    let filesystem = resource_service.0.get_filesystem(&path.mount).await?;
    let file = filesystem.get_file(&path.path).await?;
    let md = filesystem.metadata(&path.path).await?;

    let mut res = HttpResponse::build(StatusCode::OK);

    if let Some(content_type) = mime_guess::from_path(&filename).first_raw() {
        res.insert_header((header::CONTENT_TYPE, content_type));
    }

    res.insert_header((
        header::CONTENT_DISPOSITION,
        ContentDisposition {
            disposition: actix_web::http::header::DispositionType::Attachment,
            parameters: vec![
                DispositionParam::Filename(filename.to_owned()),
                DispositionParam::FilenameExt(ExtendedValue {
                    charset: Charset::Ext("UTF-8".to_owned()),
                    value: filename.as_bytes().to_vec(),
                    language_tag: None,
                }),
            ],
        },
    ));

    res.insert_header((header::ACCEPT_RANGES, "bytes"));

    if let Ok(modified) = md.modified() {
        res.insert_header((header::LAST_MODIFIED, HttpDate::from(modified).to_string()));
    }

    let mut length = md.len();
    let mut offset = 0;

    if let Some(ranges) = req.headers().get(header::RANGE) {
        if let Ok(ranges_header) = ranges.to_str() {
            if let Ok(ranges) = HttpRange::parse(ranges_header, length) {
                length = ranges[0].length;
                offset = ranges[0].start;

                if req.headers().contains_key(&header::ACCEPT_ENCODING) {
                    // don't allow compression middleware to modify partial content
                    res.insert_header((
                        header::CONTENT_ENCODING,
                        HeaderValue::from_static("identity"),
                    ));
                }

                res.insert_header((
                    header::CONTENT_RANGE,
                    format!("bytes {}-{}/{}", offset, offset + length - 1, md.len()),
                ));
            } else {
                res.insert_header((header::CONTENT_RANGE, format!("bytes */{}", length)));
                return Ok(res.status(StatusCode::RANGE_NOT_SATISFIABLE).finish());
            }
        } else {
            return Ok(res.status(StatusCode::BAD_REQUEST).finish());
        }
    }

    if offset != 0 || length != md.len() {
        res.status(StatusCode::PARTIAL_CONTENT);
    }

    let stream = file.stream(length, offset).await?;

    Ok(res.body(SizedStream::new(length, stream)))
}
