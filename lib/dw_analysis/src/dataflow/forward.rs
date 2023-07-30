use crate::controlflow::{Branch, Cfg};
use crate::dataflow::Dataflow;
use crate::errors::{AnalysisError, AnalysisResult};
use crate::repo::{Class, Method};
use dw_dex::instrs::Instr;
use dw_dex::{Addr, Dex, PrettyPrinter};
use petgraph::graph::NodeIndex;
use petgraph::visit::{DfsPostOrder, EdgeRef};
use petgraph::Direction;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;

/// The abstract state that is carried along the control flow graph
/// during forward dataflow analysis.
pub trait AbstractForwardState<'a>: Eq + Sized {
    type Context<'c>;
    type Error;

    /// The state initialization function.
    ///
    /// # Errors
    ///
    /// This method should return a `Self::Error` if given method does not allow
    /// a proper state initialization.
    fn init(method: &Method, class: &Class) -> Result<Self, Self::Error>;

    /// The state join operation function.
    ///
    /// # Errors
    ///
    /// This method should return a `Self::Error` if given methods
    /// cannot be joined properly with respect to the context.
    fn join(&mut self, other: &Self, ctx: &Self::Context<'a>) -> Result<(), Self::Error>;

    /// The control flow branch transfer function.
    ///
    /// # Errors
    ///
    /// This method should return a `Self::Error` if given branch
    /// cannot be passed with the current state with respect to the
    /// context.
    fn transfer_branch(
        &mut self,
        branch: &Branch,
        ctx: &Self::Context<'a>,
    ) -> Result<(), Self::Error>;

    /// The instruction tranfer function.
    ///
    /// # Errors
    ///
    /// This method should return a `Self::Error` if given instruction
    /// cannot be passed with the current state with respect to the
    /// context.
    fn transfer_instr(
        &mut self,
        instr: &Instr,
        dex: &Dex,
        ctx: &Self::Context<'a>,
    ) -> Result<(), Self::Error>;
}

/// Performs a forward dataflow analysis.
///
/// The analysis parameters are given by the `AbstractForwardState` trait
/// methods passed as a type parameter.
///
/// # Errors
///
/// This function may generate errors resulting of an underlying
/// abstract state error (at initialization, join or transfer
/// operation). Also, the exact type of the error is parameterized
/// through an `AbstractState` trait associated type.
pub fn forward<'a, S>(
    method: &Method,
    class: &Class,
    context: &S::Context<'a>,
) -> AnalysisResult<Dataflow<S>>
where
    S: AbstractForwardState<'a> + Clone + fmt::Display,
    S::Error: Into<AnalysisError>,
{
    let cfg = Cfg::build(method)?;
    let cfgraph = &cfg.inner;
    let dex = method.dex();

    let mut block_exits: BTreeMap<NodeIndex, S> = BTreeMap::new();
    let mut entries: BTreeMap<Addr, S> = BTreeMap::new();
    let mut exits: BTreeMap<Addr, S> = BTreeMap::new();

    // For forward dataflow, optimal order is reverse postorder.
    // The postorder here is reversed when we pop_back from the deque.
    let mut worklist: VecDeque<NodeIndex> = VecDeque::new();
    let mut postorder = DfsPostOrder::new(cfgraph, cfg.start_index());
    while let Some(id) = postorder.next(cfgraph) {
        worklist.push_back(id);
    }

    while let Some(id) = worklist.pop_back() {
        let block = &cfgraph[id];
        log::debug!("    ---- block@{}", block.start_addr());

        // retrieve list of already computed predecessors
        let preds: Vec<_> = cfgraph
            .edges_directed(id, Direction::Incoming)
            .filter(|edge| block_exits.contains_key(&edge.source()))
            .collect();

        // recompose new_state from exit states of predecessor blocks,
        let mut new_state = if preds.is_empty() {
            // when no predecessors:
            // entry = initial state
            S::init(method, class).map_err(S::Error::into)?
        } else {
            // otherwise:
            // entry = join of predecessors exits
            let mut entry: S = match preds[0].weight() {
                Branch::Catch(_) | Branch::CatchAll => {
                    let entry_addr = cfgraph[preds[0].source()].start_addr();
                    entries.get(&entry_addr).unwrap().clone()
                }
                _ => block_exits.get(&preds[0].source()).unwrap().clone(),
            };
            entry
                .transfer_branch(preds[0].weight(), context)
                .map_err(S::Error::into)?;
            for edge in preds.iter().skip(1) {
                match edge.weight() {
                    Branch::Catch(_) | Branch::CatchAll => {
                        let entry_addr = cfgraph[edge.source()].start_addr();
                        if let Some(ent) = entries.get(&entry_addr) {
                            let mut previous = ent.clone();
                            previous
                                .transfer_branch(edge.weight(), context)
                                .map_err(S::Error::into)?;
                            entry.join(&previous, context).map_err(S::Error::into)?;
                        }
                    }
                    _ => {
                        if let Some(ent) = block_exits.get(&edge.source()) {
                            let mut previous = ent.clone();
                            previous
                                .transfer_branch(edge.weight(), context)
                                .map_err(S::Error::into)?;
                            entry.join(&previous, context).map_err(S::Error::into)?;
                        }
                    }
                }
            }
            entry
        };

        log::debug!("    -- ENTRY STATE:");
        for line in format!("{new_state}").split('\n') {
            log::debug!("      {line}");
        }

        // then apply transfer function for each instruction of the block
        // while saving intermediate states
        for linstr in block.instructions() {
            entries.insert(linstr.addr(), new_state.clone());
            log::trace!("transfer_instr( {} )", PrettyPrinter(linstr.instr(), dex));
            log::trace!("    before: {new_state}");
            new_state
                .transfer_instr(linstr.instr(), dex, context)
                .map_err(S::Error::into)?;
            log::trace!("    after:  {new_state}");
            exits.insert(linstr.addr(), new_state.clone());
        }
        log::debug!("    -- EXIT STATE:");
        for line in format!("{new_state}").split('\n') {
            log::debug!("      {line}");
        }
        log::debug!("");

        // checking if need to treat again successors:
        // - if previous state was None, successors are already in worklist;
        // - if previous state was a different Some(thing), add in worklist.
        if let Some(old_state) = block_exits.get(&id) {
            if &new_state != old_state {
                cfgraph
                    .edges_directed(id, Direction::Outgoing)
                    .for_each(|edge| {
                        if !(matches!(edge.weight(), Branch::Catch(_))
                            || matches!(edge.weight(), Branch::CatchAll)
                            || worklist.contains(&edge.target()))
                        {
                            worklist.push_front(edge.target());
                        }
                    });
            }
        }

        block_exits.insert(id, new_state);
    }

    Ok(Dataflow { entries, exits })
}
