use crate::hash::{CommitHash, MigrationHash};
#[cfg(any(feature = "async", feature = "sync"))]
use crate::plan::{MigrationStep, Plan, RollbackStep};

pub trait Repo {
    type Commit: Commit;
    fn head(&self) -> CommitHash;
    fn commit(&self, commit: &CommitHash) -> Option<Self::Commit>;
    fn rollback(&self, hash: &MigrationHash) -> Option<String>;
}

pub trait Commit {
    fn commit_hash(&self) -> CommitHash;
    fn parent(&self) -> CommitHash;
    fn migration_name(&self) -> String;
    fn migration_text(&self) -> String;
    fn migration_hash(&self) -> MigrationHash;
}

#[cfg(feature = "sync")]
pub trait Apply: Repo + Sized {
    type Error;

    fn apply_migration(&mut self, step: &MigrationStep) -> Result<(), Self::Error>;
    fn apply_rollback(&mut self, step: &RollbackStep) -> Result<(), Self::Error>;
    fn commit(self) -> Result<(), Self::Error>;

    /// Apply the given plan to the database.
    fn apply(mut self, plan: &Plan) -> Result<(), Self::Error> {
        assert!(self.head() == plan.db_head);

        for rollback in &plan.rollbacks {
            self.apply_rollback(rollback)?;
        }

        for migration in &plan.migrations {
            self.apply_migration(migration)?;
        }

        self.commit()?;

        Ok(())
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncApply: Repo + Sized {
    type Error;

    async fn apply_migration(&mut self, step: &MigrationStep) -> Result<(), Self::Error>;
    async fn apply_rollback(&mut self, step: &RollbackStep) -> Result<(), Self::Error>;
    async fn commit(self) -> Result<(), Self::Error>;

    /// Apply the given plan to the database.
    async fn apply(mut self, plan: &Plan) -> Result<(), Self::Error> {
        assert!(self.head() == plan.db_head);

        for rollback in &plan.rollbacks {
            self.apply_rollback(rollback).await?;
        }

        for migration in &plan.migrations {
            self.apply_migration(migration).await?;
        }

        self.commit().await?;

        Ok(())
    }
}
