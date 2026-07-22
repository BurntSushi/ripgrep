use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use anyhow::Context;
use redb::{Database, ReadOnlyDatabase};

const INDEX_FILE_NAME: &str = "index.db";

#[derive(Debug)]
pub struct Index {
    path: PathBuf,
    db: Handle,
}

impl Index {
    pub fn builder() -> IndexBuilder {
        IndexBuilder::new()
    }

    pub fn exists(path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        path.is_dir() && path.join(INDEX_FILE_NAME).is_file()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn writer(&self) -> IndexWriter<'_> {
        IndexWriter { index: self }
    }
}

#[derive(Default)]
pub struct IndexBuilder {
    create: bool,
    write: bool,
}

impl IndexBuilder {
    pub fn new() -> IndexBuilder {
        IndexBuilder::default()
    }

    pub fn build(&self, path: impl AsRef<Path>) -> anyhow::Result<Index> {
        let path = path.as_ref();
        if self.create {
            self.build_create(path)
        } else {
            self.build_open(path)
        }
    }

    fn build_create(&self, path: &Path) -> anyhow::Result<Index> {
        if let Err(err) = std::fs::create_dir(path)
            && err.kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(err).with_context(|| {
                format!("{}: failed to create directory", path.display())
            });
        }
        let path = path.join(INDEX_FILE_NAME);
        // TODO: Retry here if the database is already open.
        let db = Database::create(&path).with_context(|| {
            format!("{}: failed to create database", path.display())
        })?;
        let index = Index { path, db: Handle::ReadWrite(db) };
        Ok(index)
    }

    fn build_open(&self, path: &Path) -> anyhow::Result<Index> {
        let path = path.join(INDEX_FILE_NAME);
        // TODO: Retry here if the database is already open.
        let db = if self.write {
            Handle::ReadWrite(Database::open(&path).with_context(|| {
                format!(
                    "{}: failed to open read-write database",
                    path.display()
                )
            })?)
        } else {
            Handle::ReadOnly(ReadOnlyDatabase::open(&path).with_context(
                || {
                    format!(
                        "{}: failed to open read-only database",
                        path.display()
                    )
                },
            )?)
        };
        let index = Index { path, db };
        Ok(index)
    }

    pub fn create(&mut self, yes: bool) -> &mut IndexBuilder {
        self.create = yes;
        self
    }

    pub fn write(&mut self, yes: bool) -> &mut IndexBuilder {
        self.write = yes;
        self
    }
}

pub struct IndexDiscovery {
    builder: IndexBuilder,
    cwd: Option<PathBuf>,
    env_name: OsString,
    dir_name: OsString,
}

impl IndexDiscovery {
    pub fn new() -> IndexDiscovery {
        IndexDiscovery {
            builder: IndexBuilder::new(),
            cwd: None,
            env_name: "RIPGREP_INDEX_PATH".into(),
            dir_name: ".ripgrep".into(),
        }
    }

    pub fn discover(&self) -> anyhow::Result<Option<Index>> {
        let cwd = match self.cwd.clone() {
            Some(cwd) => cwd,
            None => std::env::current_dir()
                .context("failed to get current working directory")?,
        };
        anyhow::ensure!(
            cwd.is_absolute(),
            "current working directory `{}` is not absolute",
            cwd.display()
        );
        if let Some(env_path) = std::env::var_os(&self.env_name) {
            if !env_path.is_empty() {
                return Ok(Some(self.builder.build(env_path)?));
            }
        }
        for ancestor in cwd.ancestors() {
            let path = ancestor.join(&self.dir_name);
            // I guess this is subject to TOCTOU problems, but the underlying
            // embedded database should handle synchronization problems for us.
            if self.builder.create || Index::exists(&path) {
                return Ok(Some(self.builder.build(&path)?));
            }
        }
        Ok(None)
    }

    pub fn cwd(&mut self, cwd: impl AsRef<Path>) -> &mut IndexDiscovery {
        self.cwd = Some(cwd.as_ref().into());
        self
    }

    pub fn env_name(
        &mut self,
        name: impl AsRef<OsStr>,
    ) -> &mut IndexDiscovery {
        self.env_name = name.as_ref().into();
        self
    }

    pub fn dir_name(
        &mut self,
        name: impl AsRef<OsStr>,
    ) -> &mut IndexDiscovery {
        self.dir_name = name.as_ref().into();
        self
    }

    pub fn create(&mut self, yes: bool) -> &mut IndexDiscovery {
        self.builder.create(yes);
        self
    }
}

enum Handle {
    ReadWrite(Database),
    ReadOnly(ReadOnlyDatabase),
}

impl Handle {
    fn begin_read(
        &self,
    ) -> Result<redb::ReadTransaction, redb::TransactionError> {
        use redb::ReadableDatabase;
        match self {
            Handle::ReadWrite(database) => database.begin_read(),
            Handle::ReadOnly(database) => database.begin_read(),
        }
    }

    fn read_write(&self) -> anyhow::Result<&Database> {
        match self {
            Handle::ReadWrite(database) => Ok(database),
            Handle::ReadOnly(_) => {
                anyhow::bail!("index was opened read-only")
            }
        }
    }

    fn read_write_mut(&mut self) -> anyhow::Result<&mut Database> {
        match self {
            Handle::ReadWrite(database) => Ok(database),
            Handle::ReadOnly(_) => {
                anyhow::bail!("index was opened read-only")
            }
        }
    }
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Handle::ReadWrite(_) => write!(f, "<redb read-write database>"),
            Handle::ReadOnly(_) => write!(f, "<redb read-only database>"),
        }
    }
}

pub struct IndexWriter<'i> {
    index: &'i Index,
}

impl<'i> IndexWriter<'i> {
    pub fn add(
        &mut self,
        path: impl AsRef<Path>,
        contents: &[u8],
    ) -> anyhow::Result<()> {
        let path = path.as_ref();
        anyhow::ensure!(
            !path.is_absolute(),
            "file paths must be relative to root"
        );
        Ok(())
    }
}
