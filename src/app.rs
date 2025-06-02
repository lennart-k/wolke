use crate::{
    dav::fs::FSResourceService,
    filesystem::SimpleFilesystemProvider,
    frontend::{FrontendConfig, configure_frontend},
};
use actix_web::{
    App, HttpResponse,
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    http::{
        Method, StatusCode,
        header::{HeaderName, HeaderValue},
    },
    middleware::{ErrorHandlerResponse, ErrorHandlers, Logger},
    web,
};
use rustical_dav::resource::ResourceService;
use std::sync::Arc;
use tracing_actix_web::TracingLogger;

pub fn make_app(
    root_path: String,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = actix_web::Error,
    >,
> {
    let fs_provider = Arc::new(SimpleFilesystemProvider::new(root_path.clone().into()));
    App::new()
        .wrap(TracingLogger::default())
        .wrap(
            ErrorHandlers::new().handler(StatusCode::METHOD_NOT_ALLOWED, |res| {
                Ok(ErrorHandlerResponse::Response(
                    if res.request().method() == Method::OPTIONS {
                        let response = HttpResponse::Ok()
                            .insert_header((
                                HeaderName::from_static("dav"),
                                // https://datatracker.ietf.org/doc/html/rfc4918#section-18
                                HeaderValue::from_static("1, 3, access-control, extended-mkcol"),
                            ))
                            .finish();
                        ServiceResponse::new(res.into_parts().0, response).map_into_right_body()
                    } else {
                        res.map_into_left_body()
                    },
                ))
            }),
        )
        .wrap(Logger::default())
        .service(
            web::scope("/mount/{mount}")
                .service(FSResourceService::new(fs_provider.clone()).actix_scope())
                .service(FSResourceService::new(fs_provider.clone()).actix_resource()),
        )
        .service(web::scope("/frontend").configure(|cfg| {
            configure_frontend(
                cfg,
                FrontendConfig {
                    secret_key: [
                        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                    ],
                },
                fs_provider.clone(),
            )
        }))
}
