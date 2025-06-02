use super::{Error, User};
use crate::filesystem::{Filesystem, FilesystemProvider};
use actix_web::{
    dev::ResourceMap,
    http::{self, header::HttpDate},
    web,
};
use async_trait::async_trait;
use derive_more::{Constructor, Deref};
use methods::{route_copy, route_delete, route_get, route_mkcol, route_move, route_put};
use rustical_dav::{
    privileges::UserPrivilegeSet,
    resource::{PrincipalUri, Resource, ResourceService},
    xml::{Resourcetype, ResourcetypeInner},
};
use rustical_xml::{EnumUnitVariants, EnumVariants, XmlDeserialize, XmlSerialize};
use serde::Deserialize;
use std::{path::PathBuf, str::FromStr, sync::Arc, time::SystemTime};

#[derive(Debug, Clone)]
pub struct FSPrincipalUri;

impl PrincipalUri for FSPrincipalUri {
    fn principal_uri(&self, principal: &str) -> String {
        format!("/mount/{principal}")
    }
}

// mod file;
mod methods;
#[derive(Debug, Clone, Deserialize)]
pub struct FSResourceServicePath {
    mount: String,
    #[serde(default)]
    path: String,
}

#[derive(Debug, Constructor, Deref)]
pub struct FSResourceService<FSP: FilesystemProvider>(Arc<FSP>);

#[async_trait(?Send)]
impl<FSP: FilesystemProvider> ResourceService for FSResourceService<FSP> {
    type MemberType = FSResource;
    type Principal = User;
    type PathComponents = FSResourceServicePath;
    type Error = Error;
    type Resource = FSResource;
    type PrincipalUri = FSPrincipalUri;

    async fn get_resource(
        &self,
        path: &Self::PathComponents,
    ) -> Result<Self::Resource, Self::Error> {
        let filesystem = self.get_filesystem(&path.mount).await?;
        let resource_path = filesystem.resolve_path(&path.path)?;
        Ok(FSResource {
            mount: path.mount.clone(),
            path: resource_path,
        })
    }

    async fn get_members(
        &self,
        path: &Self::PathComponents,
    ) -> Result<Vec<(String, Self::MemberType)>, Self::Error> {
        let filesystem = self.get_filesystem(&path.mount).await?;
        let mut result = vec![];
        let listdir = filesystem.list_dir(&path.path).await?;
        for entry in listdir.into_iter() {
            let entry = entry?;
            result.push((
                entry.file_name().into_string().unwrap(),
                FSResource {
                    path: entry.path(),
                    mount: path.mount.clone(),
                },
            ));
        }
        Ok(result)
    }

    fn actix_scope(self) -> actix_web::Scope
    where
        Self::Error: actix_web::ResponseError,
        Self::Principal: actix_web::FromRequest,
    {
        web::scope("{path:.*}").service(
            self.actix_resource()
                .get(route_get::<FSP>)
                .put(route_put::<FSP>)
                // .delete(route_delete::<FSP>)
                .route(web::method(http::Method::from_str("COPY").unwrap()).to(route_copy::<FSP>))
                .route(web::method(http::Method::from_str("MOVE").unwrap()).to(route_move::<FSP>))
                .route(
                    web::method(http::Method::from_str("MKCOL").unwrap()).to(route_mkcol::<FSP>),
                ),
        )
    }

    async fn delete_resource(
        &self,
        path: &Self::PathComponents,
        _use_trashbin: bool,
    ) -> Result<(), Self::Error> {
        let filesystem = self.0.get_filesystem(&path.mount).await?;
        filesystem.delete_file(&path.path).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct FSResource {
    pub mount: String,
    pub path: PathBuf,
}

#[derive(XmlDeserialize, XmlSerialize, PartialEq, Clone, EnumVariants, EnumUnitVariants)]
#[xml(unit_variants_ident = "FSResourcePropName")]
pub enum FSResourceProp {
    // WebDAV (RFC 4918)
    #[xml(skip_deserializing)]
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Resourcetype(Resourcetype),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Displayname(String),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Getcontentlength(u64),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Creationdate(Option<String>),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Getlastmodified(Option<String>),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Getcontenttype(Option<String>),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Getetag(Option<String>),
}

impl FSResource {
    pub fn get_content_type(&self) -> Option<&'static str> {
        mime_guess::from_path(self.path.clone()).first_raw()
    }
}

impl Resource for FSResource {
    type Prop = FSResourceProp;
    type Error = Error;
    type Principal = User;

    fn get_resourcetype(&self) -> Resourcetype {
        if self.path.is_dir() {
            Resourcetype(&[ResourcetypeInner(
                Some(rustical_dav::namespace::NS_DAV),
                "collection",
            )])
        } else {
            Resourcetype(&[])
        }
    }

    fn get_prop(
        &self,
        _puri: &impl PrincipalUri,
        _user: &User,
        prop: &FSResourcePropName,
    ) -> Result<Self::Prop, Self::Error> {
        Ok(match prop {
            FSResourcePropName::Resourcetype => {
                FSResourceProp::Resourcetype(self.get_resourcetype())
            }
            FSResourcePropName::Displayname => {
                FSResourceProp::Displayname(if let Some(file_name) = self.path.file_name() {
                    file_name.to_str().unwrap().to_owned()
                } else {
                    self.mount.to_owned()
                })
            }
            FSResourcePropName::Getcontentlength => {
                FSResourceProp::Getcontentlength(self.path.metadata().unwrap().len())
            }
            FSResourcePropName::Creationdate => FSResourceProp::Creationdate(
                self.path
                    .metadata()
                    .unwrap()
                    .created()
                    .ok()
                    .map(|system_time| HttpDate::from(system_time).to_string()),
            ),
            FSResourcePropName::Getlastmodified => FSResourceProp::Getlastmodified(
                self.path
                    .metadata()
                    .unwrap()
                    .modified()
                    .ok()
                    .map(|system_time| HttpDate::from(system_time).to_string()),
            ),
            FSResourcePropName::Getcontenttype => {
                FSResourceProp::Getcontenttype(self.get_content_type().map(|mime| mime.to_owned()))
            }
            FSResourcePropName::Getetag => FSResourceProp::Getetag(self.get_etag()),
        })
    }

    fn get_owner(&self) -> Option<&str> {
        Some(&self.mount)
    }

    fn get_user_privileges(&self, _user: &User) -> Result<UserPrivilegeSet, Self::Error> {
        Ok(UserPrivilegeSet::all())
    }

    fn get_etag(&self) -> Option<String> {
        let metadata = self.path.metadata().unwrap();
        let modified = metadata
            .modified()
            .ok()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_millis();
        let size = metadata.len();
        Some(format!("\"{size}-{modified}\""))
    }
}
