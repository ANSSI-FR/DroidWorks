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
/// during backward dataflow analysis.
pub trait AbstractBackwardState<'a>: Eq + Sized {
    type Context<'c>;
    type Error;

    /// The state initialization function.
    ///
    /// # Errors
    ///
    /// This method should return a `Self::Error` if given method does
    /// not allow a proper state initialization.
    fn init(method: &Method, class: &Class) -> Result<Self, Self::Error>;

    /// The state join operation function.
    ///
    /// # Errors
    ///
    /// This method should return a `Self::Error` if given methods
    /// cannot be met properly with respect to the context.
    fn meet(&mut self, other: &Self, ctx: &Self::Context<'a>) -> Result<(), Self::Error>;

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

    /// The instruction transfer function.
    ///
    /// # Errors
    ///
    /// This method should return a `Self::Error` if given instruction cannot be passed
    /// with the current state with respect to the context.
    fn transfer_instr(
        &mut self,
        instr: &Instr,
        dex: &Dex,
        ctx: &Self::Context<'a>,
    ) -> Result<(), Self::Error>;

    /// The entry reached function.
    ///
    /// # Errors
    ///
    /// This method should return a `Self::Error` if the state given cannot be passed
    /// at method entry.
    fn entry_reached(
        &self,
        class: &Class,
        method: &Method,
        ctx: &Self::Context<'a>,
    ) -> Result<(), Self::Error>;
}

/// Performs a backward dataflow analysis.
///
/// The analysis parameters are given by the `AbstractBackwardState`
/// trait methods passed as a type parameter.
///
/// # Errors
///
/// This function may generate errors resulting of an underlying
/// abstract state error (at initialization, meet or transfer
/// operation). Also, the exact type of the error is parameterized
/// through an `AbstractState` trait associated type.
pub fn backward<'a, S>(
    method: &Method,
    class: &Class,
    context: &S::Context<'a>,
) -> AnalysisResult<Dataflow<S>>
where
    S: AbstractBackwardState<'a> + Clone + fmt::Display,
    S::Error: Into<AnalysisError>,
{
    let cfg = Cfg::build(method)?;
    let cfgraph = &cfg.inner;
    let dex = method.dex();

    let mut block_entries: BTreeMap<NodeIndex, S> = BTreeMap::new();
    let mut entries: BTreeMap<Addr, S> = BTreeMap::new();
    let mut exits: BTreeMap<Addr, S> = BTreeMap::new();

    // For backward dataflow, optimal order is postorder.
    let mut worklist: VecDeque<NodeIndex> = VecDeque::new();
    let mut postorder = DfsPostOrder::new(cfgraph, cfg.start_index());
    while let Some(id) = postorder.next(cfgraph) {
        worklist.push_front(id);
    }

    while let Some(id) = worklist.pop_back() {
        let block = &cfgraph[id];
        log::debug!("    ---- block@{}", block.start_addr());

        // retrieve list of already computed successors
        let succs: Vec<_> = cfgraph
            .edges_directed(id, Direction::Outgoing)
            .filter(|edge| block_entries.contains_key(&edge.target()))
            .collect();

        let mut has_exception_handlers = false;

        // recompose new_state from entry states of successor blocks,
        let mut new_state = if succs.is_empty() {
            // when no successors:
            // exit = initial state
            Some(S::init(method, class).map_err(S::Error::into)?)
        } else {
            // otherwise:
            // exit = meet of successors entries
            let mut exit: Option<S> = None;
            for edge in succs.iter() {
                match edge.weight() {
                    Branch::Catch(_) | Branch::CatchAll => has_exception_handlers = true,
                    _ => {
                        if let Some(ent) = block_entries.get(&edge.target()) {
                            let mut next = ent.clone();
                            next.transfer_branch(edge.weight(), context)
                                .map_err(S::Error::into)?;
                            if let Some(exit) = &mut exit {
                                exit.meet(&next, context).map_err(S::Error::into)?;
                            } else {
                                exit = Some(next);
                            }
                        }
                    }
                }
            }
            exit
        };

        if let Some(new_state) = &mut new_state {
            if has_exception_handlers {
                log::debug!("    -- EXIT STATE (when exceptions are not thrown):");
            } else {
                log::debug!("    -- EXIT STATE:");
            }
            for line in format!("{new_state}").split('\n') {
                log::debug!("      {line}");
            }

            // then apply transfer function for each instruction of the block
            // while saving intermediate states
            for linstr in block.rev_instructions() {
                exits.insert(linstr.addr(), new_state.clone());
                log::trace!("transfer_instr( {} )", PrettyPrinter(linstr.instr(), dex));
                log::trace!("    after: {new_state}");
                new_state
                    .transfer_instr(linstr.instr(), dex, context)
                    .map_err(S::Error::into)?;
                log::trace!("    before:  {new_state}");
                entries.insert(linstr.addr(), new_state.clone());
            }

            if has_exception_handlers {
                log::debug!("    -- ENTRY STATE (when exceptions are not thrown):");
            } else {
                log::debug!("    -- ENTRY STATE:");
            }
            for line in format!("{new_state}").split('\n') {
                log::debug!("      {line}");
            }
            log::debug!("");
        }

        let new_state = if !has_exception_handlers {
            new_state.expect("internal error")
        } else {
            //entry states of successors that correspond to an exception
            //handler need to be merged with our newly computed entry
            //state (i.e. if the instruction in the current block
            //throws an exception then it is not fully executed so
            //(in normal/forard execution flow) we can jump directly
            //to the exception handler in the entry state of the current block

            for edge in succs.iter() {
                match edge.weight() {
                    Branch::Catch(_) | Branch::CatchAll => {
                        if let Some(ent) = block_entries.get(&edge.target()) {
                            let mut next = ent.clone();
                            next.transfer_branch(edge.weight(), context)
                                .map_err(S::Error::into)?;
                            if let Some(new_state) = &mut new_state {
                                new_state.meet(&next, context).map_err(S::Error::into)?;
                            } else {
                                new_state = Some(next);
                            }
                        }
                    }
                    _ => (),
                }
            }

            let new_state = new_state.expect("internal error");

            log::debug!("    -- ENTRY STATE (when exception are thrown):");
            for line in format!("{new_state}").split('\n') {
                log::debug!("      {line}");
            }
            log::debug!("");

            new_state
        };

        // checking if need to treat again predecessors:
        // - if previous state was None, predecessors are already in worklist;
        // - if previous state was a different Some(thing), add in worklist.
        if let Some(old_state) = block_entries.get(&id) {
            if &new_state != old_state {
                cfgraph
                    .edges_directed(id, Direction::Incoming)
                    .for_each(|edge| {
                        if !worklist.contains(&edge.source()) {
                            worklist.push_front(edge.source());
                        }
                    });
            }
        }

        block_entries.insert(id, new_state);
    }

    let entrypoint_state = block_entries.get(&cfg.start_index()).unwrap();

    log::debug!("    -- CHECKING ENTRY STATE");
    S::entry_reached(entrypoint_state, class, method, context).map_err(S::Error::into)?;
    log::debug!("");

    Ok(Dataflow { entries, exits })
}
