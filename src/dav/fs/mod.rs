use super::{Error, User};
use crate::{
    dav::fs::methods::{route_mkcol, route_put},
    filesystem::{DavMetadata, Filesystem, FilesystemProvider},
};
use async_trait::async_trait;
use axum::handler::Handler;
use derive_more::{Constructor, Deref};
use httpdate::HttpDate;
use methods::route_get;
use rustical_dav::{
    privileges::UserPrivilegeSet,
    resource::{
        AxumMethods, MethodFunction, PrincipalUri, Resource, ResourceName, ResourceService,
    },
    xml::{Resourcetype, ResourcetypeInner},
};
use rustical_xml::{EnumVariants, PropName, XmlDeserialize, XmlSerialize};
use scoped_fs::ScopedPath;
use serde::Deserialize;
use std::{borrow::Cow, sync::Arc, time::SystemTime};
use tower::Service;

#[derive(Debug, Clone)]
pub struct FSPrincipalUri;

impl PrincipalUri for FSPrincipalUri {
    fn principal_collection(&self) -> String {
        "/dav/mount/".into()
    }
    fn principal_uri(&self, principal: &str) -> String {
        format!("/dav/mount/{principal}/")
    }
}

mod methods;

#[derive(Debug, Clone, Deserialize)]
pub struct FSResourceServicePath {
    mount: String,
    #[serde(default)]
    path: ScopedPath,
}

#[derive(Debug, Constructor, Deref)]
pub struct FSResourceService<FSP: FilesystemProvider>(Arc<FSP>);

impl<FSP: FilesystemProvider> Clone for FSResourceService<FSP> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[async_trait]
impl<FSP: FilesystemProvider> ResourceService for FSResourceService<FSP> {
    type MemberType = FSResource<FSP>;
    type Principal = User;
    type PathComponents = FSResourceServicePath;
    type Error = Error;
    type Resource = FSResource<FSP>;
    type PrincipalUri = FSPrincipalUri;

    const DAV_HEADER: &str = "1, 3, access-control";

    async fn get_resource(
        &self,
        path: &Self::PathComponents,
        _show_deleted: bool,
    ) -> Result<Self::Resource, Self::Error> {
        let fs = self.get_filesystem(&path.mount).await?;
        let metadata = fs.metadata(&path.path).await?;
        Ok(FSResource {
            mount: path.mount.clone(),
            path: path.path.to_owned(),
            metadata,
        })
    }

    async fn get_members(
        &self,
        path: &Self::PathComponents,
    ) -> Result<Vec<Self::MemberType>, Self::Error> {
        let filesystem = self.get_filesystem(&path.mount).await?;
        let meta = filesystem.metadata(&path.path).await?;
        if !meta.is_dir() {
            return Ok(vec![]);
        }

        let mut result = vec![];
        let listdir: Vec<_> = filesystem.list_dir(&path.path).await?.into_iter().collect();
        for entry in listdir {
            result.push(FSResource {
                mount: path.mount.clone(),
                metadata: filesystem.metadata(&entry).await.unwrap(),
                path: entry,
            });
        }
        Ok(result)
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

    async fn copy_resource(
        &self,
        FSResourceServicePath { mount, path }: &Self::PathComponents,
        FSResourceServicePath {
            mount: dest_mount,
            path: dest_path,
        }: &Self::PathComponents,
        _user: &Self::Principal,
        overwrite: bool,
    ) -> Result<bool, Self::Error> {
        assert_eq!(mount, dest_mount);

        let fs = self.get_filesystem(mount).await?;
        Ok(fs.copy(path, dest_path, overwrite).await?)
    }

    async fn move_resource(
        &self,
        FSResourceServicePath { mount, path }: &Self::PathComponents,
        FSResourceServicePath {
            mount: dest_mount,
            path: dest_path,
        }: &Self::PathComponents,
        _user: &Self::Principal,
        overwrite: bool,
    ) -> Result<bool, Self::Error> {
        assert_eq!(mount, dest_mount);

        let fs = self.get_filesystem(mount).await?;
        Ok(fs.mv(path, dest_path, overwrite).await?)
    }
}

#[derive(Clone)]
pub struct FSResource<FSP: FilesystemProvider> {
    pub mount: String,
    pub path: ScopedPath,
    pub metadata: <FSP::FS as Filesystem>::Metadata,
}

impl<FSP: FilesystemProvider> ResourceName for FSResource<FSP> {
    fn get_name(&self) -> Cow<'_, str> {
        self.path.file_name().into()
    }
}

#[derive(XmlDeserialize, XmlSerialize, PartialEq, Clone, EnumVariants, PropName)]
#[xml(unit_variants_ident = "FSResourcePropName")]
pub enum FSResourceProp {
    // WebDAV (RFC 4918)
    #[xml(skip_deserializing)]
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Resourcetype(Resourcetype),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Getcontentlength(u64),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Creationdate(String),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Getlastmodified(String),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Getcontenttype(Option<String>),
    #[xml(ns = "rustical_dav::namespace::NS_DAV")]
    Getetag(Option<String>),
}

impl<FSP: FilesystemProvider> FSResource<FSP> {
    pub fn get_content_type(&self) -> Option<&'static str> {
        self.path
            .file_extension()
            .and_then(|ext| mime_guess::from_ext(ext).first_raw())
    }
}

impl<FSP: FilesystemProvider> Resource for FSResource<FSP> {
    type Prop = FSResourceProp;
    type Error = Error;
    type Principal = User;

    fn is_collection(&self) -> bool {
        self.metadata.is_dir()
    }

    fn get_resourcetype(&self) -> Resourcetype {
        if self.metadata.is_dir() {
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
            FSResourcePropName::Getcontentlength => {
                FSResourceProp::Getcontentlength(self.metadata.len())
            }
            FSResourcePropName::Creationdate => {
                FSResourceProp::Creationdate(HttpDate::from(self.metadata.created()).to_string())
            }
            FSResourcePropName::Getlastmodified => FSResourceProp::Getlastmodified(
                HttpDate::from(self.metadata.modified()).to_string(),
            ),
            FSResourcePropName::Getcontenttype => {
                FSResourceProp::Getcontenttype(self.get_content_type().map(|mime| mime.to_owned()))
            }
            FSResourcePropName::Getetag => FSResourceProp::Getetag(self.get_etag()),
        })
    }

    fn get_displayname(&self) -> Option<&str> {
        Some(self.path.file_name())
    }

    fn get_owner(&self) -> Option<&str> {
        Some(&self.mount)
    }

    fn get_user_privileges(&self, _user: &User) -> Result<UserPrivilegeSet, Self::Error> {
        Ok(UserPrivilegeSet::all())
    }

    fn get_etag(&self) -> Option<String> {
        let modified = self
            .metadata
            .modified()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_millis();
        let size = self.metadata.len();
        Some(format!("\"{size}-{modified}\""))
    }
}

impl<FSP: FilesystemProvider> AxumMethods for FSResourceService<FSP> {
    fn get() -> Option<MethodFunction<Self>> {
        Some(|state, req| {
            let mut service = Handler::with_state(route_get, state);
            Box::pin(Service::call(&mut service, req))
        })
    }

    fn put() -> Option<MethodFunction<Self>> {
        Some(|state, req| {
            let mut service = Handler::with_state(route_put, state);
            Box::pin(Service::call(&mut service, req))
        })
    }

    fn mkcol() -> Option<MethodFunction<Self>> {
        Some(|state, req| {
            let mut service = Handler::with_state(route_mkcol, state);
            Box::pin(Service::call(&mut service, req))
        })
    }
}
