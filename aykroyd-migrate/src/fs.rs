use crate::hash::{CommitHash, MigrationHash};
use crate::local::{LocalCommit, LocalRepo};

pub struct FsRepo {
    migrations_dir: std::path::PathBuf,
}

impl FsRepo {
    pub fn new<P: AsRef<std::path::Path>>(migrations_dir: P) -> Result<Self, std::io::Error> {
        let migrations_dir: std::path::PathBuf = migrations_dir.as_ref().into();
        if migrations_dir.try_exists()? {
            Ok(FsRepo { migrations_dir })
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Unable to open migrations dir {}", migrations_dir.display()),
            ))
        }
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

    pub fn migration<S: AsRef<str>>(
        &self,
        migration_name: S,
    ) -> Result<Option<FsMigration>, std::io::Error> {
        let migration_dir = self.migrations_dir.join(migration_name.as_ref());
        if migration_dir.try_exists()? {
            Ok(Some(FsMigration::new(migration_dir)))
        } else {
            Ok(None)
        }
    }

    pub fn add_migration<S: AsRef<str>>(
        &mut self,
        migration_name: S,
    ) -> Result<FsMigration, std::io::Error> {
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

    pub fn into_local(mut self) -> Result<LocalRepo, CheckError> {
        // n.b. this check validates each unwrap below
        // TODO: parse, don't validate
        // OTOH: being able to work with a half-validated structure (e.g. in guess_head) is useful
        self.check()?;

        let head = match self.head_name() {
            None => CommitHash::default(),
            Some(head_name) => {
                self.migration(head_name)
                    .map_err(CheckError::Io)?
                    .unwrap()
                    .commit()
                    .map_err(CheckError::Io)?
            }
        };

        let commits = self
            .migrations()
            .map_err(CheckError::Io)?
            .into_iter()
            .map(|migration| {
                let parent = if let Some(parent_name) = migration.parent_name()? {
                    let parent = self.migration(parent_name)?.unwrap();
                    parent.commit()?
                } else {
                    CommitHash::default()
                };
                let name = migration.name().to_string();
                let migration_text = migration.migration_text()?.unwrap_or_default();
                let rollback_text = migration.rollback_text()?;
                Ok(LocalCommit {
                    parent,
                    name,
                    migration_text,
                    rollback_text,
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(CheckError::Io)?;

        Ok(LocalRepo { head, commits })
    }

    pub fn check(&mut self) -> Result<(), CheckError> {
        for mut migration in self.migrations()? {
            migration.check_hash()?;
        }

        // TODO: this seems like a sledghammer
        if let Some(head_name) = self.head_name() {
            let head_path = self.migrations_dir.join(head_name);
            if !head_path.try_exists().map_err(CheckError::Io)? {
                std::fs::remove_file(self.head_path()).map_err(CheckError::Io)?;
            }
        }

        if self.head_name().is_none() {
            self.guess_head()?;
        }

        let mut child_name = "HEAD".to_string();
        let mut head_name = self.head_name();

        let mut to_check = vec![];

        while let Some(migration_name) = head_name {
            match self.migration(&migration_name)? {
                None => {
                    return Err(CheckError::UnknownMigration {
                        name: migration_name,
                        child: child_name,
                    })
                }
                Some(migration) => {
                    let parent = match migration.parent_name()? {
                        None => None,
                        Some(parent_name) => self.migration(&parent_name)?.map(Some).ok_or(
                            CheckError::UnknownMigration {
                                name: parent_name,
                                child: migration_name.clone(),
                            },
                        )?,
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
        // OTOH: we want to support a chain of "future" migrations when in a rollback state
        Ok(())
    }

    fn guess_head(&mut self) -> Result<(), CheckError> {
        let mut migrations = self
            .migrations()?
            .into_iter()
            .map(|m| m.name().to_string())
            .collect::<Vec<_>>();

        for migration in self.migrations()? {
            match migration.parent_name()? {
                None => {}
                Some(parent) => match migrations.iter().enumerate().find(|(_, m)| *m == &parent) {
                    Some((i, _)) => {
                        migrations.remove(i);
                    }
                    None => {
                        return Err(CheckError::UnknownMigration {
                            name: parent,
                            child: migration.name().to_string(),
                        });
                    }
                },
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
    UnknownMigration { name: String, child: String },
}

impl From<std::io::Error> for CheckError {
    fn from(err: std::io::Error) -> Self {
        CheckError::Io(err)
    }
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            CheckError::Io(err) => write!(f, "unhandled i/o error: {err}"),
            CheckError::UnknownMigration { name, child } => {
                write!(f, "missing migration {name} parent of {child}")
            }
        }
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
            let hash_s = self.hash_path().metadata()?.st_mtime();
            let hash_ns = self.hash_path().metadata()?.st_mtime_nsec();
            let parent_change = if let Some(parent) = &parent {
                let name_s = self.parent_path().metadata()?.st_mtime();
                let name_ns = self.parent_path().metadata()?.st_mtime_nsec();
                let commit_s = parent.commit_path().metadata()?.st_mtime();
                let commit_ns = parent.commit_path().metadata()?.st_mtime_nsec();
                if name_s > commit_s {
                    Some((name_s, name_ns))
                } else if commit_s > name_s {
                    Some((commit_s, commit_ns))
                } else if name_ns > commit_ns {
                    Some((name_s, name_ns))
                } else {
                    Some((commit_s, commit_ns))
                }
            } else {
                None
            };
            let commit_s = self.commit_path().metadata()?.st_mtime();
            let commit_ns = self.commit_path().metadata()?.st_mtime_nsec();

            let commit_after_hash =
                commit_s > hash_s || (commit_s == hash_s && commit_ns > hash_ns);
            let commit_after_parent = parent_change
                .map(|(parent_s, parent_ns)| {
                    commit_s > parent_s || (commit_s == parent_s && commit_ns > parent_ns)
                })
                .unwrap_or_default();

            if commit_after_hash && commit_after_parent {
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
            let name_s = self.migration_dir.metadata()?.st_ctime();
            let name_ns = self.migration_dir.metadata()?.st_ctime_nsec();
            let text_s = self.migration_text_path().metadata()?.st_mtime();
            let text_ns = self.migration_text_path().metadata()?.st_mtime_nsec();
            let hash_s = self.hash_path().metadata()?.st_mtime();
            let hash_ns = self.hash_path().metadata()?.st_mtime_nsec();

            let hash_after_name = hash_s > name_s || (hash_s == name_s && hash_ns > name_ns);
            let hash_after_text = hash_s > text_s || (hash_s == text_s && hash_ns > text_ns);

            if hash_after_name && hash_after_text {
                return Ok(());
            }
        }

        let hash = MigrationHash::from_name_and_text(
            self.name(),
            &self.migration_text()?.unwrap_or_default(),
        );
        self.set_hash(hash)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct TmpDir(std::path::PathBuf);

    impl Drop for TmpDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap()
        }
    }

    impl std::ops::Deref for TmpDir {
        type Target = std::path::Path;
        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl AsRef<std::path::Path> for TmpDir {
        fn as_ref(&self) -> &std::path::Path {
            &self.0
        }
    }

    macro_rules! tmp_dir {
        () => {{
            let now = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir()
                .join("aykroyd-tests")
                .join(format!("tst{now}-{}", line!()));
            std::fs::create_dir_all(&dir).unwrap();
            TmpDir(dir)
        }};
    }

    fn test_hash(name: &str, text: &str) {
        let dir = tmp_dir!();

        let migration_dir = dir.join(name);
        std::fs::create_dir(&migration_dir).unwrap();

        let migration_text = migration_dir.join("up.sql");
        std::fs::write(migration_text, text).unwrap();

        let mut repo = FsRepo::new(&dir).unwrap();
        repo.check().unwrap();

        let migration = repo.migration(name).unwrap().unwrap();
        let actual = migration.hash().unwrap();

        let expected = MigrationHash::from_name_and_text(name, text);

        assert_eq!(actual.to_string(), expected.to_string());
    }

    #[test]
    fn hash_simple() {
        test_hash(
            "create-table-users",
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
        );
        test_hash(
            "create-table-users",
            "CREATE TABLE users (\n  id SERIAL PRIMARY KEY,\n  name TEXT\n)",
        );
    }

    fn test_hash_update(name: &str, text: &str) {
        let dir = tmp_dir!();

        let migration_dir = dir.join(name);
        std::fs::create_dir_all(&migration_dir).unwrap();

        let migration_text = migration_dir.join("up.sql");
        std::fs::write(&migration_text, "ORIGINAL SQL TEXT").unwrap();

        let mut repo = FsRepo::new(&dir).unwrap();
        repo.check().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1));
        std::fs::write(&migration_text, text).unwrap();

        repo.check().unwrap();

        let migration = repo.migration(name).unwrap().unwrap();
        let actual = migration.hash().unwrap();

        let expected = MigrationHash::from_name_and_text(name, text);

        assert_eq!(actual.to_string(), expected.to_string());
    }

    #[test]
    fn hash_updates() {
        test_hash_update(
            "create-table-users",
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
        );
        test_hash_update(
            "create-table-users",
            "CREATE TABLE users (\n  id SERIAL PRIMARY KEY,\n  name TEXT\n)",
        );
    }

    fn test_hash_rename(name: &str, text: &str) {
        let dir = tmp_dir!();

        let migration_dir = dir.join("ORIGINAL-NAME");
        std::fs::create_dir_all(&migration_dir).unwrap();

        let migration_text = migration_dir.join("up.sql");
        std::fs::write(migration_text, text).unwrap();

        let mut repo = FsRepo::new(&dir).unwrap();
        repo.check().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1));
        std::fs::rename(migration_dir, dir.join(name)).unwrap();

        repo.check().unwrap();

        let migration = repo.migration(name).unwrap().unwrap();
        let actual = migration.hash().unwrap();

        let expected = MigrationHash::from_name_and_text(name, text);

        assert_eq!(actual.to_string(), expected.to_string());
    }

    #[test]
    fn hash_renames() {
        test_hash_rename(
            "create-table-users",
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
        );
        test_hash_rename(
            "create-table-users",
            "CREATE TABLE users (\n  id SERIAL PRIMARY KEY,\n  name TEXT\n)",
        );
    }

    fn test_commit(commits: Vec<(&str, &str)>) {
        let dir = tmp_dir!();

        let mut parent_name = "";
        let mut parent = CommitHash::default();
        let mut expecteds = vec![];

        for (name, text) in &commits {
            let migration_dir = dir.join(name);
            std::fs::create_dir(&migration_dir).unwrap();

            let migration_text = migration_dir.join("up.sql");
            std::fs::write(migration_text, text).unwrap();

            let parent_file = migration_dir.join(".parent");
            std::fs::write(parent_file, parent_name).unwrap();

            let hash = MigrationHash::from_name_and_text(name, text);
            let commit = CommitHash::from_parent_and_hash(&parent, &hash);

            expecteds.push((name, commit.clone()));

            parent = commit;
            parent_name = name;
        }
        assert_eq!(expecteds.len(), commits.len());

        let mut repo = FsRepo::new(&dir).unwrap();
        repo.check().unwrap();

        for (name, expected) in expecteds {
            let migration = repo.migration(name).unwrap().unwrap();
            let actual = migration.commit().unwrap();

            assert_eq!(actual.to_string(), expected.to_string());
        }
    }

    #[test]
    fn commit_simple() {
        test_commit(vec![(
            "create-table-users",
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
        )]);
        test_commit(vec![
            (
                "create-table-users",
                "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
            ),
            (
                "create-table-emails",
                "CREATE TABLE emails (id SERIAL PRIMARY KEY, user INT REFERENCES users)",
            ),
        ]);
        test_commit(vec![
            (
                "create-table-users",
                "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
            ),
            (
                "create-table-emails",
                "CREATE TABLE emails (id SERIAL PRIMARY KEY, user_id INT REFERENCES users)",
            ),
            (
                "add-email-column",
                "ALTER TABLE emails ADD email TEXT; UPDATE emails LEFT JOIN users ON user_id = users.id SET emails.email = users.name; ALTER COLUMN emails.email SET NOT NULL;",
            ),
        ]);
    }

    fn test_commit_edit(commits: Vec<(&str, &str)>) {
        let dir = tmp_dir!();

        let mut parent_name = "";
        let mut parent = CommitHash::default();
        let mut expecteds = vec![];

        let mut first = true;

        for (name, text) in &commits {
            let migration_dir = dir.join(name);
            std::fs::create_dir(&migration_dir).unwrap();

            let migration_text = migration_dir.join("up.sql");
            let text_to_save = if first {
                first = false;
                "INITIAL SQL TEXT"
            } else {
                text
            };
            std::fs::write(migration_text, text_to_save).unwrap();

            let parent_file = migration_dir.join(".parent");
            std::fs::write(parent_file, parent_name).unwrap();

            let hash = MigrationHash::from_name_and_text(name, text);
            let commit = CommitHash::from_parent_and_hash(&parent, &hash);

            expecteds.push((name, commit.clone()));

            parent = commit;
            parent_name = name;
        }
        assert_eq!(expecteds.len(), commits.len());

        let mut repo = FsRepo::new(&dir).unwrap();
        repo.check().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1));

        let (name, text) = &commits[0];
        let migration_dir = dir.join(name);
        let migration_text = migration_dir.join("up.sql");
        std::fs::write(migration_text, text).unwrap();

        repo.check().unwrap();

        for (name, expected) in expecteds {
            let migration = repo.migration(name).unwrap().unwrap();
            let actual = migration.commit().unwrap();

            assert_eq!(actual.to_string(), expected.to_string());
        }
    }

    #[test]
    fn commit_edit() {
        test_commit_edit(vec![(
            "create-table-users",
            "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
        )]);
        test_commit_edit(vec![
            (
                "create-table-users",
                "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
            ),
            (
                "create-table-emails",
                "CREATE TABLE emails (id SERIAL PRIMARY KEY, user INT REFERENCES users)",
            ),
        ]);
        test_commit_edit(vec![
            (
                "create-table-users",
                "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
            ),
            (
                "create-table-emails",
                "CREATE TABLE emails (id SERIAL PRIMARY KEY, user_id INT REFERENCES users)",
            ),
            (
                "add-email-column",
                "ALTER TABLE emails ADD email TEXT; UPDATE emails LEFT JOIN users ON user_id = users.id SET emails.email = users.name; ALTER COLUMN emails.email SET NOT NULL;",
            ),
        ]);
    }
}
