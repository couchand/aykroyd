use crate::hash::{CommitHash, MigrationHash};
#[cfg(any(feature = "async", feature = "sync"))]
use crate::plan::{MigrationStep, RollbackStep};
use crate::traits::{Commit, Repo};
#[cfg(feature = "async")]
use crate::traits::AsyncApply;
#[cfg(feature = "sync")]
use crate::traits::Apply;

fn escape(s: &str) -> String {
    let mut result = String::new();

    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            c => result.push(c),
        }
    }

    result
}

fn unescape(s: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut escaping = false;

    for ch in s.chars() {
        let mut new_escaping = false;
        match (escaping, ch) {
            (true, 'n') => result.push('\n'),
            (true, '\\') => result.push('\\'),
            (true, _) => return Err(format!("Unknown escape sequence '\\{ch}'")),
            (false, '\\') => new_escaping = true,
            (false, _) => result.push(ch),
        }
        escaping = new_escaping;
    }

    if escaping {
        Err("Unexpected EOF in escape sequence".into())
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn roundtrip_escape() {
        let original = "foo\nbar\n\tbaz\n\tbad\n\tgood";
        let roundtrip = unescape(&escape(original)).unwrap();
        assert_eq!(original, roundtrip);
    }

    #[test]
    fn roundtrip_serialize() {
        let original = StringFromLine::from("foo\nbar\n\tbaz\n\tbad\n\tgood");
        let serialized = format!("{:?}", original);
        let deserialized: StringFromLine = serialized.parse().unwrap();
        assert_eq!(original.0, deserialized.0);
    }

    #[test]
    fn roundtrip_serialize_evil() {
        let original = StringFromLine::from("foo\nbar\r\n\tbaz\n\r\tbad\r\tgood");
        let serialized = format!("{:?}", original);
        let deserialized: StringFromLine = serialized.parse().unwrap();
        assert_eq!(original.0, deserialized.0);
    }
}

#[derive(Clone)]
pub struct StringFromLine(String);

impl From<&str> for StringFromLine {
    fn from(s: &str) -> Self {
        StringFromLine(s.into())
    }
}

impl From<String> for StringFromLine {
    fn from(s: String) -> Self {
        StringFromLine(s)
    }
}

impl From<StringFromLine> for String {
    fn from(s: StringFromLine) -> String {
        s.0
    }
}

impl std::fmt::Debug for StringFromLine {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", escape(&self.0))
    }
}

impl std::str::FromStr for StringFromLine {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(StringFromLine(unescape(s)?))
    }
}

#[derive(Debug)]
pub struct FsRepo {
    migrations_dir: std::path::PathBuf,
    head: CommitHash,
    migrations: Vec<FsMigration>,
}

impl FsRepo {
    pub fn new<P: AsRef<std::path::Path>>(migrations_dir: P) -> Result<Self, std::io::Error> {
        let migrations_dir: std::path::PathBuf = migrations_dir.as_ref().into();
        if !migrations_dir.try_exists()? {
            std::fs::create_dir(&migrations_dir)?;
        };

        let head_path = migrations_dir.join(".head");
        let head: Option<CommitHash> = if head_path.try_exists()? {
            let head = std::fs::read_to_string(head_path)?;
            match head.parse() {
                Ok(h) => Some(h),
                Err(_) => None,
            }
        } else {
            None
        };
        let head = head.unwrap_or_default();

        let mut migrations = vec![];

        for entry in std::fs::read_dir(&migrations_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                for entry in std::fs::read_dir(path)? {
                    let entry = entry?;
                    let path = entry.path();
                    migrations.push(FsMigration::new(path)?);
                }
            }
        }

        Ok(FsRepo { migrations_dir, head, migrations })
    }

    pub fn set_head(&mut self, head: &CommitHash) -> Result<(), std::io::Error> {
        let head_file = self.migrations_dir.join(".head");
        std::fs::write(head_file, head.to_string())
    }

    pub fn create_commit(&mut self, data: FsMigrationData) -> Result<FsMigration, std::io::Error> {
        let commit_str = data.commit.to_string();
        let (first_two, rest) = commit_str.split_at(2);

        let migration_dir = self.migrations_dir.join(first_two);
        if !migration_dir.try_exists()? {
            std::fs::create_dir(&migration_dir)?;
        }

        let migration_file = migration_dir.join(rest);
        let commit = FsMigration { migration_file, data };

        commit.write()?;

        Ok(commit)
    }

    pub fn remove_commit(&mut self, commit: &CommitHash) -> Result<(), std::io::Error> {
        let commit_str = commit.to_string();
        let (first_two, rest) = commit_str.split_at(2);

        let migration_dir = self.migrations_dir.join(first_two);
        let commit_file = migration_dir.join(rest);

        std::fs::remove_file(commit_file)?;

        if std::fs::read_dir(&migration_dir)?.next().is_none() {
            std::fs::remove_dir(migration_dir)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FsMigration {
    migration_file: std::path::PathBuf,
    data: FsMigrationData,
}

#[derive(Debug, Clone)]
pub struct FsMigrationData {
    pub commit: CommitHash,
    pub parent: CommitHash,
    pub hash: MigrationHash,
    pub name: StringFromLine,
    pub text: StringFromLine,
    pub rollback: Option<StringFromLine>,
}

impl FsMigration {
    pub fn write(&self) -> Result<(), std::io::Error> {
        let mut lines = vec![];
        lines.push(self.data.commit.to_string());
        lines.push(self.data.parent.to_string());
        lines.push(self.data.hash.to_string());
        lines.push(format!("{:?}", &self.data.name));
        lines.push(format!("{:?}", &self.data.text));
        lines.push(match &self.data.rollback {
            None => "".into(),
            Some(rollback) => format!("{:?}", rollback),
        });

        let mut contents = String::new();
        for line in lines {
            contents.push_str(&line);
            contents.push('\n');
        }

        std::fs::write(&self.migration_file, contents)?; 

        Ok(())
    }

    pub fn new<P: AsRef<std::path::Path>>(migration_file: P) -> Result<Self, std::io::Error> {
        let migration_file = migration_file.as_ref().into();
        let data_raw = std::fs::read_to_string(&migration_file)?;

        let mut lines = data_raw.lines();
        macro_rules! next { () => {{
            match lines.next() {
                Some(l) => l,
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "expecting another line in FsRepo",
                    ));
                }
            }
        }}}
        macro_rules! parse { ($($inp:tt)+) => {{
            match ($($inp)+).parse() {
                Ok(o) => o,
                Err(e) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        e,
                    ));
                }
            }
        }}}
        macro_rules! maybe_parse { ($($inp:tt)+) => {{
            match ($($inp)+).as_ref() {
                "" => None,
                s => Some(parse!(s)),
            }
        }}}

        let data = FsMigrationData {
            commit: parse!(next!()),
            parent: parse!(next!()),
            hash: parse!(next!()),
            name: parse!(next!()),
            text: parse!(next!()),
            rollback: maybe_parse!(next!()),
        };

        Ok(FsMigration { migration_file, data })
    }
}

impl Repo for FsRepo {
    type Commit = FsMigration;
    fn head(&self) -> CommitHash {
        self.head.clone()
    }

    fn commit(&self, commit: &CommitHash) -> Option<Self::Commit> {
        self.migrations
            .iter()
            .find(|c| c.data.commit == *commit)
            .cloned()
    }

    fn rollback(&self, hash: &MigrationHash) -> Option<String> {
        self.migrations
            .iter()
            .find(|c| c.data.hash == *hash)
            .and_then(|c| c.data.rollback.clone().map(Into::into))
    }
}

impl Commit for FsMigration {
    fn commit_hash(&self) -> CommitHash {
        self.data.commit.clone()
    }

    fn parent(&self) -> CommitHash {
        self.data.parent.clone()
    }

    fn migration_name(&self) -> String {
        self.data.name.clone().into()
    }

    fn migration_text(&self) -> String {
        self.data.text.clone().into()
    }

    fn migration_hash(&self) -> MigrationHash {
        self.data.hash.clone()
    }
}

#[cfg(feature = "sync")]
impl Apply for FsRepo {
    type Error = std::io::Error;

    fn apply_migration(&mut self, step: &MigrationStep) -> Result<(), std::io::Error> {
        self.create_commit(FsMigrationData {
            commit: step.commit(),
            parent: step.parent.clone(),
            hash: step.hash(),
            name: step.name.into(),
            text: step.text.into(),
            rollback: step.rollback.into(),
        })?;
        self.set_head(&step.commit())?;

        Ok(())
    }

    fn apply_rollback(&mut self, step: &RollbackStep) -> Result<(), std::io::Error> {
        self.set_head(&step.parent)?;
        self.remove_commit(&step.commit())?;

        Ok(())
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl AsyncApply for FsRepo {
    type Error = std::io::Error;

    async fn apply_migration(&mut self, step: &MigrationStep) -> Result<(), std::io::Error> {
        // TODO: use tokio async fs probably!
        self.create_commit(FsMigrationData {
            commit: step.commit(),
            parent: step.parent.clone(),
            hash: step.hash(),
            name: step.name.clone().into(),
            text: step.text.clone().into(),
            rollback: step.rollback.clone().map(Into::into),
        })?;
        self.set_head(&step.commit())?;

        Ok(())
    }

    async fn apply_rollback(&mut self, step: &RollbackStep) -> Result<(), std::io::Error> {
        self.set_head(&step.parent)?;
        self.remove_commit(&step.commit())?;

        Ok(())
    }
}
