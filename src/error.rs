//! Error types for DFA JIT compilation.

/// Errors from DFA compilation or execution.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The DFA has zero states.
    #[error("DFA has zero states. Fix: provide at least one state in the transition table.")]
    EmptyDfa,

    /// The transition table dimensions are inconsistent.
    #[error("invalid transition table: {reason}")]
    InvalidTable {
        /// Description of the inconsistency.
        reason: String,
    },

    /// Executable memory allocation failed.
    #[error("failed to allocate executable memory: {reason}. Fix: check OS memory limits and mmap permissions.")]
    MemoryAllocation {
        /// Underlying reason.
        reason: String,
    },

    /// The DFA state count exceeds the JIT compiler's limit.
    #[error("DFA has {states} states, exceeding the {max}-state JIT limit. Fix: use the interpreted fallback for large DFAs.")]
    TooManyStates {
        /// Actual state count.
        states: usize,
        /// Maximum supported by JIT.
        max: usize,
    },

    /// The JIT scanner produced different results than the interpreted scanner
    /// during the per-table self-check parity pass.
    #[error("JIT self-check parity failed: {reason}. Fix: this is a dfajit compiler bug; report it and use the interpreted fallback.")]
    JitParity {
        /// Description of the mismatch.
        reason: String,
    },
}

/// Result type alias.
pub type Result<T> = std::result::Result<T, Error>;
