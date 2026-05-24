use crate::error::{Error, Result};

use super::TransitionTable;

impl TransitionTable {
    /// Serialize the transition table to bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let accept_count = self.accept_states.len();
        let pat_len_count = self.pattern_lengths.len();
        let size = 4usize
            .checked_add(4)
            .and_then(|s| s.checked_add(self.transitions.len().checked_mul(4)?))
            .and_then(|s| s.checked_add(4))
            .and_then(|s| s.checked_add(accept_count.checked_mul(8)?))
            .and_then(|s| s.checked_add(4))
            .and_then(|s| s.checked_add(pat_len_count.checked_mul(4)?))
            .unwrap_or(8);
        let mut buf = Vec::with_capacity(size);
        buf.extend_from_slice(&(self.state_count as u32).to_le_bytes());
        buf.extend_from_slice(&(self.class_count as u32).to_le_bytes());
        for &t in &self.transitions {
            buf.extend_from_slice(&t.to_le_bytes());
        }
        buf.extend_from_slice(&(accept_count as u32).to_le_bytes());
        for &(state, pid) in &self.accept_states {
            buf.extend_from_slice(&state.to_le_bytes());
            buf.extend_from_slice(&pid.to_le_bytes());
        }
        buf.extend_from_slice(&(pat_len_count as u32).to_le_bytes());
        for &l in &self.pattern_lengths {
            buf.extend_from_slice(&l.to_le_bytes());
        }
        buf
    }

    /// Deserialize a transition table from bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are truncated or contain invalid data.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(Error::InvalidTable {
                reason: "data too short for header. Fix: provide at least 8 bytes for state_count and class_count.".into(),
            });
        }
        let state_count = u32::from_le_bytes(data[0..4].try_into().unwrap_or([0; 4])) as usize;
        let class_count = u32::from_le_bytes(data[4..8].try_into().unwrap_or([0; 4])) as usize;
        let trans_len = state_count
            .checked_mul(class_count)
            .ok_or_else(|| Error::InvalidTable {
                reason: "state_count * class_count overflow. Fix: reduce state_count or class_count to fit in usize.".into(),
            })?;
        let trans_bytes = trans_len
            .checked_mul(4)
            .ok_or_else(|| Error::InvalidTable {
                reason:
                    "transition table byte length overflow. Fix: reduce state_count or class_count."
                        .into(),
            })?;
        let trans_end = 8usize
            .checked_add(trans_bytes)
            .ok_or_else(|| Error::InvalidTable {
                reason:
                    "transition table end offset overflow. Fix: reduce state_count or class_count."
                        .into(),
            })?;
        if data.len() < trans_end + 4 {
            return Err(Error::InvalidTable {
                reason: "truncated transition table. Fix: provide the full transition array."
                    .into(),
            });
        }

        let mut transitions = Vec::with_capacity(trans_len);
        for i in 0..trans_len {
            let off = 8 + i * 4;
            let val = u32::from_le_bytes(data[off..off + 4].try_into().unwrap_or([0; 4]));
            transitions.push(val);
        }

        let accept_count =
            u32::from_le_bytes(data[trans_end..trans_end + 4].try_into().unwrap_or([0; 4]))
                as usize;
        let accept_bytes = accept_count
            .checked_mul(8)
            .ok_or_else(|| Error::InvalidTable {
                reason:
                    "accept states byte length overflow. Fix: reduce the number of accept states."
                        .into(),
            })?;
        let mut pos = trans_end + 4;
        if data.len()
            < pos
                .checked_add(accept_bytes)
                .ok_or_else(|| Error::InvalidTable {
                    reason: "accept states end offset overflow. Fix: reduce the number of accept states.".into(),
                })?
        {
            return Err(Error::InvalidTable {
                reason: "truncated accept states. Fix: provide the full accept_states block.".into(),
            });
        }

        let mut accept_states = Vec::with_capacity(accept_count);
        for _ in 0..accept_count {
            let state = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap_or([0; 4]));
            let pid = u32::from_le_bytes(data[pos + 4..pos + 8].try_into().unwrap_or([0; 4]));
            accept_states.push((state, pid));
            pos += 8;
        }

        if pos + 4 > data.len() {
            return Err(Error::InvalidTable {
                reason:
                    "truncated pattern lengths header. Fix: provide the full pattern_lengths block."
                        .into(),
            });
        }
        let pat_count =
            u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap_or([0; 4])) as usize;
        let pat_bytes = pat_count
            .checked_mul(4)
            .ok_or_else(|| Error::InvalidTable {
                reason: "pattern lengths byte length overflow. Fix: reduce the number of pattern lengths.".into(),
            })?;
        pos += 4;
        if data.len()
            < pos
                .checked_add(pat_bytes)
                .ok_or_else(|| Error::InvalidTable {
                    reason: "pattern lengths end offset overflow. Fix: reduce the number of pattern lengths.".into(),
                })?
        {
            return Err(Error::InvalidTable {
                reason: "truncated pattern lengths. Fix: provide the full pattern_lengths block.".into(),
            });
        }

        let mut pattern_lengths = Vec::with_capacity(pat_count);
        for _ in 0..pat_count {
            let l = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap_or([0; 4]));
            pattern_lengths.push(l);
            pos += 4;
        }

        Self::from_parts(
            state_count,
            class_count,
            transitions,
            accept_states,
            pattern_lengths,
        )
    }

    /// Construct a transition table from validated parts.
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions are inconsistent or out of bounds.
    pub fn from_parts(
        state_count: usize,
        class_count: usize,
        transitions: Vec<u32>,
        accept_states: Vec<(u32, u32)>,
        pattern_lengths: Vec<u32>,
    ) -> Result<Self> {
        if state_count > Self::MAX_STATES {
            return Err(Error::TooManyStates {
                states: state_count,
                max: Self::MAX_STATES,
            });
        }
        if class_count == 0 {
            return Err(Error::InvalidTable {
                reason: "class_count must be greater than 0. Fix: pass a positive class_count."
                    .into(),
            });
        }
        let expected_len = state_count
            .checked_mul(class_count)
            .ok_or_else(|| Error::InvalidTable {
                reason: "state_count * class_count overflow. Fix: reduce state_count or class_count to fit in usize.".into(),
            })?;
        if transitions.len() != expected_len {
            return Err(Error::InvalidTable {
                reason: format!(
                    "transition table has {} entries but expected {}. Fix: ensure transitions.len() == state_count * class_count.",
                    transitions.len(),
                    expected_len,
                ),
            });
        }
        for &t in &transitions {
            let state = t & 0x7FFF_FFFF;
            if state as usize >= state_count {
                return Err(Error::InvalidTable {
                    reason: format!(
                        "transition target state {state} exceeds state count {state_count}. Fix: ensure all transition targets are < state_count."
                    ),
                });
            }
        }

        let pat_len = pattern_lengths.len();
        let mut seen_states = vec![false; state_count];
        for &(state, pid) in &accept_states {
            if state as usize >= state_count {
                return Err(Error::InvalidTable {
                    reason: format!("accept state {state} exceeds state count {state_count}. Fix: ensure all accept states are < state_count."),
                });
            }
            if seen_states[state as usize] {
                return Err(Error::InvalidTable {
                    reason: format!(
                        "state {state} has multiple accept patterns, which is not supported. Fix: merge patterns or split into separate states."
                    ),
                });
            }
            seen_states[state as usize] = true;
            if pid as usize >= pat_len {
                return Err(Error::InvalidTable {
                    reason: format!("pattern ID {pid} in accept states has no length defined. Fix: call set_pattern_length({pid}, len) before compiling."),
                });
            }
        }

        Ok(Self {
            state_count,
            class_count,
            transitions,
            accept_states,
            pattern_lengths,
        })
    }
}
