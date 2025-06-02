use std::{
    cmp,
    fs::{File, Metadata},
    io::{Read, SeekFrom},
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

use actix_web::{ResponseError, http::StatusCode, web::Bytes};
use async_trait::async_trait;
use futures::Stream;
use std::fs::DirEntry;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("Not Found")]
    NotFound,
}

impl ResponseError for Error {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[async_trait(?Send)]
pub trait FilesystemProvider: 'static {
    async fn get_filesystem(&self, mount: &str) -> Result<impl Filesystem, Error>;
}

#[async_trait(?Send)]
pub trait FileReader {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error>;
    async fn stream(
        self,
        len: u64,
        offset: u64,
    ) -> Result<impl Stream<Item = Result<Bytes, Error>>, Error>;
}

#[async_trait(?Send)]
impl FileReader for std::fs::File {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        std::io::Seek::seek(&mut self, pos)
    }
    async fn stream(mut self, len: u64, offset: u64) -> Result<FileStream, Error> {
        FileReader::seek(&mut self, SeekFrom::Start(offset)).await?;
        Ok(FileStream::new(self, len, offset))
    }
}

struct FileStream {
    file: std::fs::File,
    len: u64,
    offset: u64,
    counter: u64,
}

impl FileStream {
    fn new(file: std::fs::File, len: u64, offset: u64) -> Self {
        Self {
            file,
            len,
            offset,
            counter: 0,
        }
    }
}

impl Stream for FileStream {
    type Item = Result<Bytes, Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        dbg!(self.offset, self.len, self.counter);
        if self.len <= self.counter {
            Poll::Ready(None)
        } else {
            let max_bytes = cmp::min(self.len - self.counter, 65_536) as usize;
            let mut buf = vec![0u8; max_bytes];
            self.file.read_exact(&mut buf)?;
            self.counter += max_bytes as u64;
            Poll::Ready(Some(Ok(Bytes::from(buf))))
        }
    }
}

#[async_trait(?Send)]
pub trait Filesystem: 'static {
    type FileReader: FileReader;

    async fn metadata(&self, path: &str) -> Result<Metadata, Error>;
    fn resolve_path(&self, path: &str) -> Result<PathBuf, Error>;
    async fn get_file(&self, path: &str) -> Result<Self::FileReader, Error>;
    async fn delete_file(&self, path: &str) -> Result<(), Error>;
    async fn list_dir(
        &self,
        path: &str,
    ) -> Result<impl IntoIterator<Item = Result<DirEntry, std::io::Error>>, Error>;
    async fn create_dir(&self, path: &str) -> Result<(), Error>;
    async fn create_file(&self, path: &str) -> Result<File, Error>;
    async fn copy(&self, from: &str, to: &str) -> Result<(), Error>;
    async fn mv(&self, from: &str, to: &str) -> Result<(), Error>;
}

pub struct SimpleFilesystemProvider {
    root_path: PathBuf,
}

impl SimpleFilesystemProvider {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }
}

#[async_trait(?Send)]
impl FilesystemProvider for SimpleFilesystemProvider {
    async fn get_filesystem(&self, mount: &str) -> Result<impl Filesystem, Error> {
        let sub_path = self.root_path.join(mount);
        assert!(sub_path.starts_with(&self.root_path));
        Ok(SimpleFilesystem {
            root_path: self.root_path.join(mount),
        })
    }
}

pub struct SimpleFilesystem {
    root_path: PathBuf,
}

#[async_trait(?Send)]
impl Filesystem for SimpleFilesystem {
    type FileReader = std::fs::File;

    async fn metadata(&self, path: &str) -> Result<Metadata, Error> {
        let sub_path = self.root_path.join(path);
        assert!(sub_path.starts_with(&self.root_path));
        Ok(sub_path.metadata()?)
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, Error> {
        let sub_path = self.root_path.join(path);
        assert!(sub_path.starts_with(&self.root_path));
        if !sub_path.try_exists()? {
            return Err(Error::NotFound);
        }
        Ok(sub_path)
    }

    async fn get_file(&self, path: &str) -> Result<Self::FileReader, Error> {
        let sub_path = self.root_path.join(path);
        assert!(sub_path.starts_with(&self.root_path));
        if !sub_path.try_exists()? {
            return Err(Error::NotFound);
        }
        let file = std::fs::File::open(sub_path)?;

        Ok(file)
    }

    async fn delete_file(&self, path: &str) -> Result<(), Error> {
        let sub_path = self.root_path.join(path);
        assert!(sub_path.starts_with(&self.root_path));
        dbg!(&sub_path);

        if sub_path.is_file() {
            std::fs::remove_file(&sub_path)?;
        }
        if sub_path.is_dir() {
            // TODO: Use remove_dir_all
            std::fs::remove_dir(&sub_path)?;
        }

        Ok(())
    }

    async fn list_dir(&self, path: &str) -> Result<Vec<Result<DirEntry, std::io::Error>>, Error> {
        let sub_path = self.root_path.join(path);
        assert!(sub_path.starts_with(&self.root_path));
        if !sub_path.is_dir() {
            return Ok(vec![]);
        }
        Ok(std::fs::read_dir(&sub_path)?.collect())
    }

    async fn create_dir(&self, path: &str) -> Result<(), Error> {
        let sub_path = self.root_path.join(path);
        assert!(sub_path.starts_with(&self.root_path));
        println!("\n\n\n\n\n\n\n");
        dbg!(&sub_path);
        Ok(std::fs::create_dir(&sub_path)?)
    }

    async fn create_file(&self, path: &str) -> Result<File, Error> {
        let sub_path = self.root_path.join(path);
        assert!(sub_path.starts_with(&self.root_path));

        Ok(File::create(sub_path)?)
    }

    async fn copy(&self, from: &str, to: &str) -> Result<(), Error> {
        let sub_path_from = self.root_path.join(from);
        let sub_path_to = self.root_path.join(to);
        assert!(sub_path_from.starts_with(&self.root_path));
        assert!(sub_path_to.starts_with(&self.root_path));
        std::fs::copy(sub_path_from, sub_path_to)?;
        Ok(())
    }

    async fn mv(&self, from: &str, to: &str) -> Result<(), Error> {
        let sub_path_from = self.root_path.join(from);
        let sub_path_to = self.root_path.join(to);
        assert!(sub_path_from.starts_with(&self.root_path));
        assert!(sub_path_to.starts_with(&self.root_path));
        std::fs::rename(sub_path_from, sub_path_to)?;
        Ok(())
    }
}
