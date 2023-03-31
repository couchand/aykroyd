//! Local migrations are the ones that are in your project directory.

use crate::hash::MigrationHash;
use crate::Error;

#[derive(Debug)]
pub struct LocalMigration {
    pub file: std::path::PathBuf,
    pub text: String,
    pub hash: MigrationHash,
}

impl LocalMigration {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<LocalMigration, Error> {
        let file = path.as_ref().to_path_buf();
        println!("Loading migration from {}", file.display());

        let text = std::fs::read_to_string(&file).map_err(Error::io_error)?;
        let hash = MigrationHash::from_content(&text);
        println!("  - {hash}");

        Ok(LocalMigration { file, text, hash })
    }
}

#[derive(Debug)]
pub struct LocalCommit {
    pub dir: std::path::PathBuf,
    pub up: LocalMigration,
    pub down: Option<LocalMigration>,
    pub up_deps: Vec<MigrationHash>,
    pub down_deps: Vec<MigrationHash>,
    pub up_hash: MigrationHash,
    pub down_hash: MigrationHash,
}

impl LocalCommit {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<LocalCommit, Error> {
        let dir = path.as_ref().to_path_buf();
        println!("Loading commit from {}", dir.display());

        let up_file = dir.join("up.sql");
        let up_deps_file = dir.join("up.deps");

        let mut up_deps = std::fs::read_to_string(&up_deps_file)
            .map_err(Error::io_error)?
            .lines()
            .map(str::trim)
            .map(str::parse)
            .collect::<Result<Vec<_>, _>>()?;
        if up_deps.is_empty() {
            up_deps.push(MigrationHash::ZERO);
        }

        println!("UP Dependencies:");
        for dep in &up_deps {
            println!("  - {dep}");
        }

        let up = LocalMigration::load(&up_file)?;
        let up_hash = MigrationHash::from_deps_and_hash(&up_deps, &up.hash);

        println!("UP Hash:");
        println!("  - {up_hash}");

        let down_file = dir.join("down.sql");
        let down_deps_file = dir.join("down.deps");

        let mut down_deps = std::fs::read_to_string(&down_deps_file)
            .or_else(|e| match e.kind() {
                std::io::ErrorKind::NotFound => Ok(String::new()),
                _ => Err(e),
            })
            .map_err(Error::io_error)?
            .lines()
            .map(str::trim)
            .map(str::parse)
            .collect::<Result<Vec<_>, _>>()?;
        if down_deps.is_empty() {
            down_deps.push(MigrationHash::ZERO);
        }

        println!("DN Dependencies:");
        for dep in &down_deps {
            println!("  - {dep}");
        }

        let down = LocalMigration::load(down_file).ok();
        let down_hash =
            MigrationHash::from_deps_and_hash_opt(&down_deps, down.as_ref().map(|m| &m.hash));

        println!("DN Hash:");
        println!("  - {down_hash}");

        Ok(LocalCommit {
            dir,
            up,
            down,
            up_deps,
            down_deps,
            up_hash,
            down_hash,
        })
    }
}

#[derive(Debug)]
pub struct LocalRepo {
    dir: std::path::PathBuf,
    commits: std::collections::HashMap<MigrationHash, LocalCommit>,
}

impl LocalRepo {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> Result<LocalRepo, Error> {
        let dir = path.as_ref().to_path_buf();
        println!("Loading migrations from {}", dir.display());

        let mut commits = std::collections::HashMap::new();

        for entry in std::fs::read_dir(&dir).map_err(Error::io_error)? {
            let entry = entry.map_err(Error::io_error)?;
            let path = entry.path();
            println!("Dir entry: {}", path.display());

            if path.is_dir() {
                let commit = LocalCommit::load(path)?;
                commits.insert(commit.up_hash.clone(), commit);
            } else {
                println!("Not a dir... maybe load without deps???");
            }
        }

        // If you specify up deps but not down deps, your down hash is wrong. Fix it.
        let mut fixes = vec![];
        for (hash, commit) in commits.iter() {
            #[allow(clippy::collapsible_if)]
            if matches!(&commit.down_deps[..], &[MigrationHash::ZERO]) {
                if !matches!(&commit.up_deps[..], &[MigrationHash::ZERO]) {
                    println!("Down tree needs fixing");
                    println!("  {hash}");

                    let mut down_deps = vec![];

                    for dep in &commit.up_deps {
                        match commits.get(dep) {
                            None => return Err(Error::unable_to_fix_down_tree(&hash.to_string())), // TODO: make this skippable
                            Some(dep) => down_deps.push(dep.down_hash.clone()),
                        }
                    }

                    let down_hash = MigrationHash::from_deps_and_hash_opt(
                        &down_deps,
                        commit.down.as_ref().map(|m| &m.hash),
                    );
                    fixes.push((hash.clone(), down_hash));
                }
            }
        }
        for fix in fixes {
            commits.get_mut(&fix.0).unwrap().down_hash = fix.1;
        }

        Ok(LocalRepo { dir, commits })
    }

    pub fn get(&self, hash: &MigrationHash) -> Option<&LocalCommit> {
        self.commits.get(hash)
    }

    pub fn take(&mut self, hash: &MigrationHash) -> Option<LocalCommit> {
        self.commits.remove(hash)
    }

    pub fn dir(&self) -> &std::path::Path {
        &self.dir
    }

    pub fn iter(&self) -> impl Iterator<Item = &LocalCommit> {
        self.commits.values()
    }
}
