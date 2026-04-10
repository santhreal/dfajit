#![allow(clippy::missing_panics_doc)]
//! JIT compilation of DFA transition tables to native x86_64 code.
//!
//! `dfajit` converts a DFA state machine (as produced by `warpstate`) into
//! a native function that scans input bytes without table lookup indirection.
//! Each DFA state becomes a labeled basic block with a 256-entry jump table
//! indexed by the input byte.
//!
//! # Architecture
//!
//! The compiled function has this signature:
//!
//! ```text
//! fn(input: *const u8, len: usize, matches: *mut Match, max_matches: usize) -> usize
//! ```
//!
//! Returns the number of matches written. The function:
//! 1. Loads the start state
//! 2. For each input byte: loads byte, indexes into jump table, jumps to next state
//! 3. At accept states: writes match to output buffer, increments count
//! 4. Returns match count
//!
//! # Example
//!
//! ```rust
//! use dfajit::{TransitionTable, JitDfa};
//!
//! // Build a simple 3-state DFA that matches "ab"
//! let mut table = TransitionTable::new(3, 256).unwrap();
//! table.set_transition(0, b'a', 1);  // state 0 --'a'--> state 1
//! table.set_transition(1, b'b', 2);  // state 1 --'b'--> state 2
//! table.add_accept(2, 0);            // state 2 accepts pattern 0
//! // All other transitions go to state 0 (dead/restart)
//!
//! let jit = JitDfa::compile(&table).unwrap();
//! // Pass sufficiently large array slice since JIT doesn't re-allocate
//! let mut matches = vec![matchkit::Match::from_parts(0, 0, 0); 10];
//! let count = jit.scan(b"xabxab", &mut matches);
//! assert_eq!(count, 2);
//! ```

#![warn(missing_docs, clippy::pedantic)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::todo,
        clippy::unimplemented,
        clippy::panic
    )
)]
#![allow(
    clippy::assigning_clones,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::needless_range_loop,
    clippy::ptr_as_ptr,
    clippy::similar_names,
    clippy::too_many_lines
)]

mod codegen;
mod dfa;
mod error;
mod table;

pub use dfa::JitDfa;
pub use error::{Error, Result};
pub use table::TransitionTable;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests_core;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests_advanced;
