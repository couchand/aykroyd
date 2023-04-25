//! Embedded migrations are the ones within your released app to migrate the production database.

use crate::Error;
use crate::hash::MigrationHash;

#[derive(Debug, Clone, Copy)]
pub struct EmbeddedMigration {
    pub name: &'static str,
    pub deps: &'static [&'static str],
    pub text: &'static str,
}

impl EmbeddedMigration {
    pub fn text_hash(&self) -> MigrationHash {
        MigrationHash::from_content(&self.text)
    }

    pub fn hash(&self) -> Result<MigrationHash, Error> {
        Ok(MigrationHash::from_deps_and_hash(&self.deps()?, &self.text_hash()))
    }

    pub fn deps(&self) -> Result<Vec<MigrationHash>, Error> {
        self.deps.iter().map(|dep| dep.parse()).collect()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddedMigrations {
    pub migrations: &'static [EmbeddedMigration],
}

#[derive(Debug)]
pub struct EmbeddedRepo {
    commits: std::collections::HashMap<MigrationHash, EmbeddedMigration>,
}

impl EmbeddedRepo {
    pub fn load(embedded: &EmbeddedMigrations) -> Result<EmbeddedRepo, Error> {
        let commits = embedded.migrations
            .iter()
            .map(|migration| migration.hash().map(|hash| (hash, *migration)))
            .collect::<Result<_, _>>()?;

        Ok(EmbeddedRepo { commits })
    }

    pub fn contains(&self, hash: &MigrationHash) -> bool {
        self.commits.contains_key(hash)
    }

    pub fn get(&self, hash: &MigrationHash) -> Option<&EmbeddedMigration> {
        self.commits.get(hash)
    }

    pub fn take(&mut self, hash: &MigrationHash) -> Option<EmbeddedMigration> {
        self.commits.remove(hash)
    }

    pub fn iter(&self) -> impl Iterator<Item = &EmbeddedMigration> {
        self.commits.values()
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
        let repo_dir = self.dir.unwrap_or_else(|| std::path::PathBuf::from("./migrations"));

        assert!(repo_dir.exists(), "Unable to find migration directory: {}", repo_dir.display());

        println!("cargo:rerun-if-changed={}", repo_dir.display());

        let out_file = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join(
            self.output.unwrap_or_else(|| std::path::PathBuf::from("akroyd-migrations.rs"))
        );
        let repo = crate::local::LocalRepo::load(&repo_dir).unwrap();

        let mut code = String::new();

        code.push_str("::akroyd_migrate::embedded::EmbeddedMigrations {\n");
        code.push_str("    migrations: &[\n");

        for migration in repo.iter() {
            code.push_str("        ::akroyd_migrate::embedded::EmbeddedMigration {\n");

            code.push_str("            name: ");
            code.push_str(&format!("{:?}", migration.dir.file_name().unwrap()));
            code.push_str(",\n");

            code.push_str("            deps: &[");
            for dep in &migration.up_deps {
                code.push_str(&format!("{:?}", dep.to_string()));
            }
            code.push_str("],\n");

            code.push_str("            text: ");
            code.push_str(&format!("{:?}", migration.up.text));
            code.push_str(",\n");

            code.push_str("        },\n");
        }

        code.push_str("    ]\n");
        code.push_str("}\n");

        std::fs::write(out_file, code)
    }
}

#[macro_export]
macro_rules! include_migrations {
    (
    ) => {
        include!(concat!(env!("OUT_DIR"), "/akroyd-migrations.rs"));
    };
    (
        $filename:literal
    ) => {
        include!(concat!(env!("OUT_DIR"), "/", $filename));
    };
}
