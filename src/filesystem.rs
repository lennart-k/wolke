use async_trait::async_trait;
use futures::Stream;
use http::StatusCode;
use scoped_fs::ScopedPath;
use std::fs::DirEntry;
use std::time::SystemTime;
use std::{
    cmp,
    fs::File,
    io::{Read, SeekFrom},
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IO(std::io::Error),
    #[error("Not Found")]
    NotFound,
    #[error("Conflict")]
    Conflict,
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        if value.kind() == std::io::ErrorKind::NotFound {
            Self::NotFound
        } else {
            Self::IO(value)
        }
    }
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::IO(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
        }
    }
}

#[async_trait]
pub trait FilesystemProvider: Clone + Send + Sync + 'static {
    type FS: Filesystem;

    async fn get_filesystem(&self, mount: &str) -> Result<Self::FS, Error>;
}

#[async_trait]
pub trait FileReader {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error>;
    async fn stream(
        self,
        len: u64,
        offset: u64,
    ) -> Result<impl Stream<Item = Result<Vec<u8>, Error>> + Send, Error>;
}

#[async_trait]
impl FileReader for std::fs::File {
    async fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        std::io::Seek::seek(&mut self, pos)
    }
    async fn stream(mut self, len: u64, offset: u64) -> Result<FileStream, Error> {
        FileReader::seek(&mut self, SeekFrom::Start(offset)).await?;
        Ok(FileStream::new(self, len))
    }
}

pub struct FileStream {
    file: std::fs::File,
    len: u64,
    counter: u64,
}

impl FileStream {
    fn new(file: std::fs::File, len: u64) -> Self {
        Self {
            file,
            len,
            counter: 0,
        }
    }
}

impl Stream for FileStream {
    type Item = Result<Vec<u8>, Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.len <= self.counter {
            Poll::Ready(None)
        } else {
            let max_bytes = cmp::min(self.len - self.counter, 65_536) as usize;
            let mut buf = vec![0u8; max_bytes];
            self.file.read_exact(&mut buf)?;
            self.counter += max_bytes as u64;
            Poll::Ready(Some(Ok(buf)))
        }
    }
}

pub trait DavMetadata: Clone + Send + Sync + 'static {
    fn len(&self) -> u64;
    fn modified(&self) -> SystemTime;
    fn created(&self) -> SystemTime;
    fn is_dir(&self) -> bool;
}

#[async_trait]
pub trait Filesystem: Clone + Send + Sync + 'static {
    type FileReader: FileReader;
    type Metadata: DavMetadata;

    async fn metadata(&self, path: &ScopedPath) -> Result<Self::Metadata, Error>;
    async fn get_file(&self, path: &ScopedPath) -> Result<Self::FileReader, Error>;
    async fn delete_file(&self, path: &ScopedPath) -> Result<(), Error>;
    async fn list_dir(
        &self,
        path: &ScopedPath,
    ) -> Result<impl IntoIterator<Item = ScopedPath>, Error>;
    async fn create_dir(&self, path: &ScopedPath) -> Result<(), Error>;
    async fn create_file(&self, path: &ScopedPath) -> Result<File, Error>;
    async fn copy(
        &self,
        from: &ScopedPath,
        to: &ScopedPath,
        overwrite: bool,
    ) -> Result<bool, Error>;
    async fn mv(&self, from: &ScopedPath, to: &ScopedPath, overwrite: bool) -> Result<bool, Error>;
}

#[derive(Clone)]
pub struct SimpleFilesystemProvider {
    root_path: PathBuf,
}

impl SimpleFilesystemProvider {
    pub fn new(root_path: PathBuf) -> Self {
        Self { root_path }
    }
}

#[async_trait]
impl FilesystemProvider for SimpleFilesystemProvider {
    type FS = SimpleFilesystem;

    async fn get_filesystem(&self, mount: &str) -> Result<Self::FS, Error> {
        let sub_path = self.root_path.join(mount);
        assert!(sub_path.starts_with(&self.root_path));
        Ok(SimpleFilesystem {
            root_path: self.root_path.join(mount),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SimpleFilesystem {
    root_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SimpleFilesystemMetadata(std::fs::Metadata);

impl DavMetadata for SimpleFilesystemMetadata {
    fn len(&self) -> u64 {
        self.0.len()
    }

    fn modified(&self) -> SystemTime {
        self.0.modified().unwrap()
    }

    fn created(&self) -> SystemTime {
        self.0.created().unwrap()
    }

    fn is_dir(&self) -> bool {
        self.0.is_dir()
    }
}

#[async_trait]
impl Filesystem for SimpleFilesystem {
    type FileReader = std::fs::File;
    type Metadata = SimpleFilesystemMetadata;

    async fn metadata(&self, path: &ScopedPath) -> Result<Self::Metadata, Error> {
        let ospath = path.with_base(&self.root_path);
        Ok(SimpleFilesystemMetadata(ospath.metadata()?))
    }

    async fn get_file(&self, path: &ScopedPath) -> Result<Self::FileReader, Error> {
        let ospath = path.with_base(&self.root_path);
        if !ospath.is_file() {
            return Err(Error::NotFound);
        }
        let file = std::fs::File::open(ospath)?;
        Ok(file)
    }

    async fn delete_file(&self, path: &ScopedPath) -> Result<(), Error> {
        let ospath = path.with_base(&self.root_path);

        if ospath.is_file() {
            std::fs::remove_file(&ospath)?;
        }
        if ospath.is_dir() {
            // TODO: Use remove_dir_all
            std::fs::remove_dir(&ospath)?;
        }

        Ok(())
    }

    async fn list_dir(&self, path: &ScopedPath) -> Result<Vec<ScopedPath>, Error> {
        let ospath = path.with_base(&self.root_path);
        Ok(std::fs::read_dir(&ospath)?
            .collect::<Result<Vec<DirEntry>, _>>()?
            .into_iter()
            .map(|entry| path.join_segment(entry.file_name().to_str().unwrap()))
            .collect())
    }

    async fn create_dir(&self, path: &ScopedPath) -> Result<(), Error> {
        let ospath = path.with_base(&self.root_path);
        Ok(std::fs::create_dir(&ospath)?)
    }

    async fn create_file(&self, path: &ScopedPath) -> Result<File, Error> {
        let ospath = path.with_base(&self.root_path);
        Ok(File::create(ospath)?)
    }

    async fn copy(
        &self,
        from: &ScopedPath,
        to: &ScopedPath,
        overwrite: bool,
    ) -> Result<bool, Error> {
        let ospath_from = from.with_base(&self.root_path);
        let ospath_to = to.with_base(&self.root_path);
        let exists = ospath_to.exists();
        if exists && !overwrite {
            return Err(Error::Conflict);
        }
        std::fs::copy(&ospath_from, &ospath_to)?;
        Ok(exists)
    }

    async fn mv(&self, from: &ScopedPath, to: &ScopedPath, overwrite: bool) -> Result<bool, Error> {
        let ospath_from = from.with_base(&self.root_path);
        let ospath_to = to.with_base(&self.root_path);
        let exists = ospath_to.exists();
        if exists && !overwrite {
            return Err(Error::Conflict);
        }
        std::fs::rename(&ospath_from, &ospath_to)?;
        Ok(exists)
    }
}
