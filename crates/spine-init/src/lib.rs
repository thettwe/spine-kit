//! `spine init` — the first command written, and the first that can brick a
//! repository.
//!
//! PB §6.7: "`spine init` writes files into someone else's repository: a CI
//! workflow, a managed block in `AGENTS.md`, a keyring, a `.gitignore` entry.
//! Every toolkit that does this without a lifecycle dies the same way … The fix
//! is the one package managers settled on: a lockfile, hashes, and a refusal to
//! overwrite what you did not write."
//!
//! There is no upgrade command: on an initialised repository `init` is
//! idempotent, and an upgrade is a re-run.

pub mod abort;
pub mod apply;
pub mod git;
pub mod plan;
pub mod resolve;
pub mod rollback;
pub mod staging;
pub mod uninstall;
pub mod upgrade;

pub use abort::{AbortError, Aborted};
pub use apply::{Applied, ApplyError};
pub use git::{HeadTree, Repo};
pub use plan::{Action, Plan, PlanRow, RefuseReason, State, TreeSource};
pub use resolve::{Reclassification, Resolutions, ResolveError, Resolved};
pub use rollback::{RestoreAction, RollbackError, RollbackPlan};
pub use staging::{Interrupted, Staging};
pub use uninstall::{UninstallAction, UninstallError, UninstallPlan};
pub use upgrade::{Endpoint, UpgradeError, UpgradeLine};
