//! Dataflow analysis framework.

use dw_dex::Addr;
use std::collections::BTreeMap;

mod backward;
mod forward;

pub use backward::{backward, AbstractBackwardState};
pub use forward::{forward, AbstractForwardState};

/// Dataflow analysis result object.
///
/// Contains entries and exits abstract states for every basic block
/// of the analyzed method, after reaching fixpoint.
#[derive(Debug, Clone)]
pub struct Dataflow<S> {
    pub entries: BTreeMap<Addr, S>,
    pub exits: BTreeMap<Addr, S>,
}
