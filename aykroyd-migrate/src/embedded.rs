//! Embedded migrations are the ones within your released app to migrate the production database.

use crate::fs::FsRepo;
use crate::hash::{CommitHash, MigrationHash};
use crate::local::{LocalCommit, LocalRepo};
use crate::traits::{Commit, Repo};

#[derive(Debug, Clone, Copy)]
pub struct EmbeddedMigration {
    pub parent: &'static str,
    pub name: &'static str,
    pub text: &'static str,
    pub rollback: Option<&'static str>,
}

impl EmbeddedMigration {
    pub fn hash(&self) -> MigrationHash {
        MigrationHash::from_name_and_text(self.name, self.text)
    }

    pub fn parent(&self) -> CommitHash {
        if self.parent.is_empty() {
            CommitHash::default()
        } else {
            self.parent.parse().unwrap()
        }
    }

    pub fn commit(&self) -> CommitHash {
        CommitHash::from_parent_and_hash(&self.parent(), &self.hash())
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddedRepo {
    pub head: &'static str,
    pub migrations: &'static [EmbeddedMigration],
}

impl EmbeddedRepo {
    pub fn load(&self) -> LocalRepo {
        let head = self.head.parse().unwrap();
        let commits = self
            .migrations
            .iter()
            .map(|migration| LocalCommit {
                parent: migration.parent(),
                name: migration.name.to_string(),
                migration_text: migration.text.to_string(),
                rollback_text: migration.rollback.map(|s| s.to_string()),
            })
            .collect();

        LocalRepo { head, commits }
    }
}

#[derive(Debug)]
pub struct EmbeddedRepoBuilder {
    dir: Option<std::path::PathBuf>,
    output: Option<std::path::PathBuf>,
}
impl EmbeddedRepoBuilder {
    pub fn new() -> Self {
        EmbeddedRepoBuilder {
            dir: None,
            output: None,
        }
    }

    pub fn with_dir<P: AsRef<std::path::Path>>(mut self, dir: P) -> Self {
        self.dir = Some(dir.as_ref().to_path_buf());
        self
    }

    pub fn with_output<P: AsRef<std::path::Path>>(mut self, output: P) -> Self {
        self.output = Some(output.as_ref().to_path_buf());
        self
    }

    pub fn build(self) -> Result<(), std::io::Error> {
        let repo_dir = self
            .dir
            .unwrap_or_else(|| std::path::PathBuf::from("./.myg"));

        assert!(
            repo_dir.exists(),
            "Unable to find migration directory: {}",
            repo_dir.display()
        );

        println!("cargo:rerun-if-changed={}", repo_dir.display());

        let out_file = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join(
            self.output
                .unwrap_or_else(|| std::path::PathBuf::from("aykroyd-migrations.rs")),
        );
        let repo = FsRepo::new(&repo_dir).unwrap();

        let mut code = String::new();

        code.push_str("::aykroyd_migrate::embedded::EmbeddedRepo {\n");

        code.push_str("    head: ");
        code.push_str(&format!("{:?}", repo.head().to_string()));
        code.push_str(",\n");

        code.push_str("    migrations: &[\n");

        let mut cursor = repo.head();
        while !cursor.is_zero() {
            let commit = match repo.commit(&cursor) {
                Some(c) => c,
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unable to find commit {cursor}"),
                    ));
                }
            };

            code.push_str("        ::aykroyd_migrate::embedded::EmbeddedMigration {\n");

            code.push_str("            parent: ");
            code.push_str(&format!("{:?}", commit.parent().to_string()));
            code.push_str(",\n");

            code.push_str("            name: ");
            code.push_str(&format!("{:?}", commit.migration_name()));
            code.push_str(",\n");

            code.push_str("            text: ");
            code.push_str(&format!("{:?}", commit.migration_text()));
            code.push_str(",\n");

            code.push_str("            rollback: ");
            code.push_str(&format!("{:?}", repo.rollback(&commit.migration_hash())));
            code.push_str(",\n");

            code.push_str("        },\n");

            cursor = commit.parent();
        }

        code.push_str("    ]\n");
        code.push_str("}\n");

        std::fs::write(out_file, code)
    }
}

impl Default for EmbeddedRepoBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[macro_export]
macro_rules! include_migrations {
    (
    ) => {
        include!(concat!(env!("OUT_DIR"), "/aykroyd-migrations.rs"));
    };
    (
        $filename:literal
    ) => {
        include!(concat!(env!("OUT_DIR"), "/", $filename));
    };
}
