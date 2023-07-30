//! This crate provides Android application bytecode analysis algorithm for
//! the `DroidWorks` project.

pub mod callgraph;
pub mod controlflow;
pub mod dataflow;
pub mod errors;
pub mod hierarchy;
pub mod information_flow;
pub mod repo;
pub mod stats;
pub mod typing;

use crate::errors::AnalysisResult;

pub fn forward_typecheck(
    method: &repo::Method,
    class: &repo::Class,
    repo: &repo::Repo,
) -> AnalysisResult<typing::Types> {
    typing::Types::forward_compute(method, class, repo)
}

pub fn backward_typecheck(
    method: &repo::Method,
    class: &repo::Class,
    repo: &repo::Repo,
) -> AnalysisResult<typing::Types> {
    typing::Types::backward_compute(method, class, repo)
}
