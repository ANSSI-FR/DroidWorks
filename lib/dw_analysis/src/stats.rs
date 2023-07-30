//! Utils functions to make simple heuristics on functions code.

use crate::errors::AnalysisResult;
use dw_dex::code::CodeItem;
use dw_dex::instrs::Instr;
use dw_dex::{Dex, DexIndex};
use std::collections::BTreeSet;

// Stubbed method looks like this:
//    0000: invoke-direct {v2}, java/lang/Object-><init>()V
//    0003: new-instance v0, Ljava/lang/RuntimeException;
//    0005: const-string v1, "Stub!"
//    0007: invoke-direct {v0, v1}, java/lang/RuntimeException-><init>(Ljava/lang/String;)V
//    0010: throw v0
//
// Determination of stubbed method relies on instructions number and
// the presence of the string "Stub!".
pub(crate) fn is_stub(code: &CodeItem, dex: &Dex) -> AnalysisResult<bool> {
    let icount = code.instructions_count();
    let is_stubbed = if icount < 10 {
        let strings = used_strings(code, dex)?;
        strings.contains("Stub!")
    } else {
        false
    };
    Ok(is_stubbed)
}

pub(crate) fn used_strings(code: &CodeItem, dex: &Dex) -> AnalysisResult<BTreeSet<String>> {
    let mut strings = BTreeSet::new();
    for instr in code.iter_instructions() {
        match instr.instr() {
            Instr::ConstString(_, s) | Instr::ConstStringJumbo(_, s) => {
                strings.insert(s.get(dex)?.to_string(dex)?);
            }
            _ => (),
        }
    }
    Ok(strings)
}
