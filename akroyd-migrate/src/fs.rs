use crate::hash2::{CommitHash, MigrationHash};

pub struct FsRepo {
    migrations_dir: std::path::PathBuf,
}

// TODO: this is a bad idea
impl std::fmt::Debug for FsRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut migrations = vec![];
        for migration in self.migrations().unwrap() {
            migrations.push(format!(
                "{name} - {commit} - {hash}\n    {text}",
                name = migration.name(),
                commit = migration.commit().unwrap(),
                hash = migration.hash().unwrap(),
                text = migration.migration_text().unwrap().unwrap_or_default().replace('\n', "\n    "),
            ));
        }
        writeln!(
            f,
            "{dir}/\n  HEAD: {head}\n  - {migrations}",
            dir = self.migrations_dir.display(),
            head = self.head_name().as_ref().map(AsRef::as_ref).unwrap_or("<no commits>"),
            migrations = migrations.join("\n  - "),
        )
    }
}

impl FsRepo {
    pub fn new<P: AsRef<std::path::Path>>(migrations_dir: P) -> Self {
        let migrations_dir = migrations_dir.as_ref().into();
        FsRepo { migrations_dir }
    }

    fn head_path(&self) -> std::path::PathBuf {
        self.migrations_dir.join(".head")
    }

    pub fn head_name(&self) -> Option<String> {
        std::fs::read_to_string(self.head_path()).ok()
    }

    pub fn set_head_name<S: AsRef<str>>(&mut self, head_name: S) -> Result<(), std::io::Error> {
        std::fs::write(self.head_path(), head_name.as_ref())
    }

    pub fn migration<S: AsRef<str>>(&self, migration_name: S) -> Result<Option<FsMigration>, std::io::Error> {
        let migration_dir = self.migrations_dir.join(migration_name.as_ref());
        if migration_dir.try_exists()? {
            Ok(Some(FsMigration::new(migration_dir)))
        } else {
            Ok(None)
        }
    }

    pub fn add_migration<S: AsRef<str>>(&mut self, migration_name: S) -> Result<FsMigration, std::io::Error> {
        let migration_dir = self.migrations_dir.join(migration_name.as_ref());
        std::fs::create_dir(&migration_dir)?;
        Ok(FsMigration::new(migration_dir))
    }

    pub fn migrations(&self) -> Result<Vec<FsMigration>, std::io::Error> {
        let mut migrations = vec![];

        for entry in std::fs::read_dir(&self.migrations_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                migrations.push(FsMigration::new(path));
            }
        }

        Ok(migrations)
    }

    pub fn commit(&mut self, migration: &mut FsMigration) -> Result<(), std::io::Error> {
        let head_name = self.head_name().unwrap_or_default();
        migration.set_parent_name(&head_name)?;
        self.set_head_name(migration.name())?;
        Ok(())
    }

    pub fn check(&mut self) -> Result<(), CheckError> {
        for mut migration in self.migrations()? {
            migration.check_hash()?;
        }

        if self.head_name().is_none() {
            self.guess_head()?;
        }

        let mut child_name = "HEAD".to_string();
        let mut head_name = self.head_name();

        let mut to_check = vec![];

        while let Some(migration_name) = head_name {
            match self.migration(&migration_name)? {
                None => return Err(CheckError::UnknownMigration {
                    name: migration_name,
                    child: child_name,
                }),
                Some(migration) => {
                    let parent = match migration.parent_name()? {
                        None => None,
                        Some(parent_name) => {
                            self.migration(&parent_name)?
                                .map(Some)
                                .ok_or(CheckError::UnknownMigration {
                                    name: parent_name.into(),
                                    child: migration_name.clone(),
                                })?
                        }
                    };

                    let parent_name = migration.parent_name()?.clone();

                    to_check.push((migration, parent));

                    child_name = migration_name;
                    head_name = parent_name;
                }
            }
        }

        // n.b. we need to calculate parent commit hash before child
        to_check.reverse();

        for (mut migration, parent) in to_check {
            migration.check_commit(parent)?;
        }

        // TODO: check uncommitted migrations are parentless
        Ok(())
    }

    fn guess_head(&mut self) -> Result<(), CheckError> {
        let mut migrations = self.migrations()?
            .into_iter()
            .map(|m| m.name().to_string())
            .collect::<Vec<_>>();

        for migration in self.migrations()? {
            match migration.parent_name()? {
                None => {}
                Some(parent) => {
                    match migrations.iter().enumerate().find(|(_, m)| *m == &parent) {
                        Some((i, _)) => {
                            migrations.remove(i);
                        }
                        None => {
                            return Err(CheckError::UnknownMigration {
                                name: parent,
                                child: migration.name().to_string(),
                            });
                        }
                    }
                }
            }
        }

        if migrations.len() == 1 {
            self.set_head_name(&migrations[0])?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum CheckError {
    Io(std::io::Error),
    UnknownMigration {
        name: String,
        child: String,
    },
}

impl From<std::io::Error> for CheckError {
    fn from(err: std::io::Error) -> Self {
        CheckError::Io(err)
    }
}

pub struct FsMigration {
    migration_dir: std::path::PathBuf,
}

impl FsMigration {
    pub fn new<P: AsRef<std::path::Path>>(migration_dir: P) -> Self {
        let migration_dir = migration_dir.as_ref().into();
        FsMigration { migration_dir }
    }

    fn parent_path(&self) -> std::path::PathBuf {
        self.migration_dir.join(".parent")
    }

    fn migration_text_path(&self) -> std::path::PathBuf {
        self.migration_dir.join("up.sql")
    }

    pub fn migration_text(&self) -> Result<Option<String>, std::io::Error> {
        let path = self.migration_text_path();
        if path.try_exists()? {
            std::fs::read_to_string(&path).map(Some)
        } else {
            Ok(None)
        }
    }

    fn rollback_text_path(&self) -> std::path::PathBuf {
        self.migration_dir.join("down.sql")
    }

    pub fn rollback_text(&self) -> Result<Option<String>, std::io::Error> {
        let path = self.rollback_text_path();
        if path.try_exists()? {
            std::fs::read_to_string(&path).map(Some)
        } else {
            Ok(None)
        }
    }

    fn hash_path(&self) -> std::path::PathBuf {
        self.migration_dir.join(".hash")
    }

    fn set_hash(&mut self, hash: MigrationHash) -> Result<(), std::io::Error> {
        std::fs::write(self.hash_path(), hash.to_string())
    }

    pub fn hash(&self) -> Result<MigrationHash, std::io::Error> {
        std::fs::read_to_string(self.hash_path())?
            .parse()
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }

    fn commit_path(&self) -> std::path::PathBuf {
        self.migration_dir.join(".commit")
    }

    fn set_commit(&mut self, commit: CommitHash) -> Result<(), std::io::Error> {
        std::fs::write(self.commit_path(), commit.to_string())
    }

    pub fn commit(&self) -> Result<CommitHash, std::io::Error> {
        std::fs::read_to_string(self.commit_path())?
            .parse()
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
    }

    pub fn is_committed(&self) -> Result<bool, std::io::Error> {
        self.parent_path().try_exists()
    }

    pub fn parent_name(&self) -> Result<Option<String>, std::io::Error> {
        let path = self.parent_path();
        if path.try_exists()? {
            let s = std::fs::read_to_string(path)?;
            let s = s.trim();

            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(s.into()))
            }
        } else {
            Ok(None)
        }
    }

    pub fn set_parent_name<S: AsRef<str>>(&mut self, parent_name: S) -> Result<(), std::io::Error> {
        std::fs::write(self.parent_path(), parent_name.as_ref())
    }

    pub fn name(&self) -> &str {
        self.migration_dir
            .file_name()
            .unwrap() // path cannot end with ..
            .to_str()
            .unwrap() // path must be Unicode
    }

    pub fn check_commit(&mut self, parent: Option<FsMigration>) -> Result<(), std::io::Error> {
        use std::os::linux::fs::MetadataExt;
        if self.commit_path().exists() {
            let hash_change = self.hash_path().metadata()?.st_mtime();
            let parent_change = if let Some(parent) = &parent {
                let name_change = self.parent_path().metadata()?.st_mtime();
                let commit_change = parent.commit_path().metadata()?.st_mtime();
                Some(name_change.max(commit_change))
            } else {
                None
            };
            let commit_change = self.commit_path().metadata()?.st_mtime();

            if commit_change >= hash_change && parent_change.map(|change| commit_change >= change).unwrap_or_default() {
                return Ok(());
            }
        }
        let parent_commit = parent.map(|m| m.commit()).transpose()?.unwrap_or_default();
        let commit = CommitHash::from_parent_and_hash(&parent_commit, &self.hash()?);
        self.set_commit(commit)
    }

    pub fn check_hash(&mut self) -> Result<(), std::io::Error> {
        use std::os::linux::fs::MetadataExt;
        if self.hash_path().exists() {
            let name_change = self.migration_dir.metadata()?.st_ctime();
            let text_change = self.migration_text_path().metadata()?.st_mtime();
            let hash_change = self.hash_path().metadata()?.st_mtime();
            
            if hash_change >= name_change && hash_change >= text_change {
                return Ok(());
            }
        }
        
        let hash = MigrationHash::from_name_and_text(self.name(), &self.migration_text()?.unwrap_or_default());
        self.set_hash(hash)
    }
}
