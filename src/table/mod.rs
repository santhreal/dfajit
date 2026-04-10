use crate::error::{Error, Result};

mod analysis;
mod minimize;
mod serialize;

/// DFA transition table consumed by the JIT compiler.
#[derive(Debug, Clone)]
pub struct TransitionTable {
    state_count: usize,
    class_count: usize,
    transitions: Vec<u32>,
    accept_states: Vec<(u32, u32)>,
    pattern_lengths: Vec<u32>,
}

impl TransitionTable {
    /// Maximum states allowed in a single DFA.
    pub const MAX_STATES: usize = 65_536;

    /// Create a new empty transition table.
    ///
    /// # Errors
    ///
    /// Returns [`Error::TooManyStates`] if `state_count` exceeds [`Self::MAX_STATES`].
    ///
    /// # Panics
    ///
    /// Panics if `state_count * class_count` would overflow `usize`.
    pub fn new(state_count: usize, class_count: usize) -> Result<Self> {
        if state_count > Self::MAX_STATES {
            return Err(Error::TooManyStates {
                states: state_count,
                max: Self::MAX_STATES,
            });
        }
        if class_count == 0 {
            return Err(Error::InvalidTable {
                reason: "class_count must be greater than 0. Fix: pass a positive class_count.".into(),
            });
        }
        let total = state_count
            .checked_mul(class_count)
            .ok_or(Error::TooManyStates {
                states: state_count,
                max: Self::MAX_STATES,
            })?;
        const MAX_TOTAL_TRANSITIONS: usize = 256 * 1024 * 1024;
        if total > MAX_TOTAL_TRANSITIONS {
            return Err(Error::TooManyStates {
                states: state_count,
                max: Self::MAX_STATES,
            });
        }
        Ok(Self {
            state_count,
            class_count,
            transitions: vec![0; total],
            accept_states: Vec::new(),
            pattern_lengths: Vec::new(),
        })
    }

    /// Set a single transition: from `state` on input `byte`, go to `next_state`.
    pub fn set_transition(&mut self, state: usize, byte: u8, next_state: u32) {
        let idx = state * self.class_count + byte as usize;
        debug_assert!(idx < self.transitions.len());
        if idx < self.transitions.len() {
            self.transitions[idx] = next_state;
        }
    }

    /// Mark a state as accepting for a given pattern.
    pub fn add_accept(&mut self, state: u32, pattern_id: u32) {
        self.accept_states.push((state, pattern_id));
        if self.pattern_lengths.len() <= pattern_id as usize {
            self.pattern_lengths.resize(pattern_id as usize + 1, 0);
        }
    }

    /// Set the fixed length for a pattern (used to compute match start).
    pub fn set_pattern_length(&mut self, pattern_id: u32, length: u32) {
        if self.pattern_lengths.len() <= pattern_id as usize {
            self.pattern_lengths.resize(pattern_id as usize + 1, 0);
        }
        self.pattern_lengths[pattern_id as usize] = length;
    }

    /// Number of DFA states.
    #[must_use]
    pub fn state_count(&self) -> usize {
        self.state_count
    }

    /// Number of input classes.
    #[must_use]
    pub fn class_count(&self) -> usize {
        self.class_count
    }

    /// Transition array slice.
    #[must_use]
    pub fn transitions(&self) -> &[u32] {
        &self.transitions
    }

    /// Mutable transition array.
    pub fn transitions_mut(&mut self) -> &mut Vec<u32> {
        &mut self.transitions
    }

    /// Accept state metadata slice.
    #[must_use]
    pub fn accept_states(&self) -> &[(u32, u32)] {
        &self.accept_states
    }

    /// Mutable accept states vector.
    pub fn accept_states_mut(&mut self) -> &mut Vec<(u32, u32)> {
        &mut self.accept_states
    }

    /// Pattern lengths slice.
    #[must_use]
    pub fn pattern_lengths(&self) -> &[u32] {
        &self.pattern_lengths
    }

    /// Mutable pattern lengths vector.
    pub fn pattern_lengths_mut(&mut self) -> &mut Vec<u32> {
        &mut self.pattern_lengths
    }

    /// Number of transitions in the table.
    #[must_use]
    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }
}
