//! Wrappers over `dw_dex` raw structures (classes, methods, etc.)
//! to enrich them and store them in the repository.

mod class;
mod field;
mod method;
mod repository;
mod uids;

pub use class::Class;
pub use field::Field;
pub use method::Method;
pub use repository::Repo;
pub use uids::{ClassUid, FieldUid, MethodUid, RepoCounters};

pub(crate) use method::MethodDescr;
