use crate::{
    dav::{
        Error,
        fs::{FSResourceService, FSResourceServicePath},
    },
    filesystem::{DavMetadata, FileReader, Filesystem, FilesystemProvider},
};
use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
};
use axum_extra::TypedHeader;
use headers::Range;
use http::{HeaderValue, Request, StatusCode, header};
use httpdate::HttpDate;
use percent_encoding::{CONTROLS, percent_encode};
use rustical_dav::resource::ResourceService;
use std::ops::Bound;

pub async fn route_get<FSP: FilesystemProvider>(
    State(resource_service): State<FSResourceService<FSP>>,
    Path(path): Path<FSResourceServicePath>,
    http_range: Option<TypedHeader<Range>>,
    req: Request<Body>,
) -> Result<Response<Body>, Error> {
    let resource = resource_service.get_resource(&path, false).await?;
    let filename = resource.path.file_name();
    let filename = percent_encode(filename.as_bytes(), CONTROLS).to_string();
    let filesystem = resource_service.0.get_filesystem(&path.mount).await?;
    let md = filesystem.metadata(&path.path).await?;
    let file = filesystem.get_file(&path.path).await?;

    let mut res = Response::builder().status(StatusCode::OK);
    let headers = res.headers_mut().unwrap();

    if let Some(content_type) = mime_guess::from_path(&filename).first_raw() {
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    }

    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachement; filename*=UTF-8''{}; filename={}",
            filename, filename
        ))
        .unwrap(),
    );

    headers.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));

    headers.insert(
        header::LAST_MODIFIED,
        HeaderValue::try_from(HttpDate::from(md.modified()).to_string()).unwrap(),
    );

    let mut length = md.len();
    let mut offset = 0;

    if let Some(TypedHeader(range_header)) = http_range {
        let mut ranges = range_header.satisfiable_ranges(length);
        if let Some((start, end)) = ranges.next() {
            offset = match start {
                Bound::Unbounded => 0,
                Bound::Included(start) => start,
                _ => {
                    return Ok(res
                        .status(StatusCode::RANGE_NOT_SATISFIABLE)
                        .body(Body::empty())
                        .unwrap());
                }
            };
            length = match end {
                Bound::Unbounded => length,
                Bound::Included(end) => end,
                Bound::Excluded(end) => end - 1,
            } - offset;
        }
        if ranges.next().is_some() {
            // We have more than one range
            return Ok(res
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .body(Body::empty())
                .unwrap());
        }

        if req.headers().contains_key(&header::ACCEPT_ENCODING) {
            // don't allow compression middleware to modify partial content
            headers.insert(
                header::CONTENT_ENCODING,
                HeaderValue::from_static("identity"),
            );
        }

        headers.insert(
            header::CONTENT_RANGE,
            HeaderValue::try_from(format!(
                "bytes {}-{}/{}",
                offset,
                offset + length - 1,
                md.len()
            ))
            .unwrap(),
        );
    }

    if offset != 0 || length != md.len() {
        res = res.status(StatusCode::PARTIAL_CONTENT);
    }

    let stream = file.stream(length, offset).await?;

    Ok(res.body(Body::from_stream(stream)).unwrap())
}
