//! Dalvik bytecode typing pass stuff.

mod backward;
mod forward;
mod types;

pub mod errors;

use crate::dataflow;
use crate::dataflow::Dataflow;
use crate::errors::AnalysisResult;
use crate::repo::{Class, Method, Repo};
use crate::typing::errors::TypeError;
use crate::typing::types::AbstractType;
use dw_dex::registers::Reg;
use std::fmt;

/// Result of the typing pass.
///
/// Contains abstract types information for registers at entries and exits of every basic block
/// of the analyzed method.
pub type Types = Dataflow<State>;

impl Types {
    /// Runs a forward typechecking pass onto given method and
    /// corresponding control flow graph, and returns results of the
    /// dataflow analysis.
    ///
    /// # Errors
    ///
    /// This function may generate errors, mainly due to typecheck error (breaking subtyping
    /// type usage, etc.), but also due to internal error (register out of bounds, etc.).
    pub fn forward_compute(method: &Method, class: &Class, context: &Repo) -> AnalysisResult<Self> {
        dataflow::forward(method, class, context)
    }

    /// Runs a backward typechecking pass onto given method and
    /// corresponding control flow graph, and returns results of the
    /// dataflow analysis.
    ///
    /// # Errors
    ///
    /// This function may generate errors, mainly due to typecheck
    /// error (breaking subtyping type usage, etc.), but also due to
    /// internal error (register out of bounds, etc.).
    pub fn backward_compute(
        method: &Method,
        class: &Class,
        context: &Repo,
    ) -> AnalysisResult<Self> {
        dataflow::backward(method, class, context)
    }
}

macro_rules! tc {
    ( $t1:ident <: $t2:expr ; $repo:expr ) => {
        $t1.is_subseteq($t2, $repo)
    };
}
pub(crate) use tc;

/// The abstract state for the typing pass.
///
/// Contains abstract types of registers and special abstract register type
/// information for interprocedural exchanges (exception handling, method
/// invocation return types).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    registers: Vec<AbstractType>,
    last_exception: Option<AbstractType>,
    last_result: Option<AbstractType>,
    expected: Option<AbstractType>,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..self.registers.len() {
            write!(f, "    v{i}: {}", self.registers[i])?;
        }
        if let Some(v) = &self.last_exception {
            write!(f, "    last_exception: {v}")?;
        }
        if let Some(v) = &self.last_result {
            write!(f, "    last_result: {v}")?;
        }
        Ok(())
    }
}

impl State {
    /// Returns the type of the rth register.
    ///
    /// # Errors
    ///
    /// This function may return an out of bounds error.
    pub fn read_reg(&self, r: Reg) -> AnalysisResult<&AbstractType> {
        self.registers
            .get(r.value() as usize)
            .ok_or_else(|| TypeError::OutOfBoundsRegister(r).into())
    }

    fn read_pair(&self, r: Reg) -> AnalysisResult<&AbstractType> {
        let t1 = self.read_reg(r)?;
        let t2 = self.read_reg(r.next())?;
        if t1 == t2 {
            Ok(t1)
        } else {
            Err(TypeError::BadPairTypes {
                type1: format!("{t1}"),
                type2: format!("{t2}"),
            }
            .into())
        }
    }

    fn write_reg(&mut self, r: Reg, t: AbstractType) -> AnalysisResult<()> {
        self.registers
            .get_mut(r.value() as usize)
            .map(|rt| *rt = t)
            .ok_or_else(|| TypeError::OutOfBoundsRegister(r).into())
    }

    fn write_pair(&mut self, r: Reg, t: AbstractType) -> AnalysisResult<()> {
        self.write_reg(r, t.clone())?;
        self.write_reg(r.next(), t)
    }
}
