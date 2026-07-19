//! x86_64 JIT codegen buffer and runtime scanning helpers.

#[cfg(target_arch = "x86_64")]
use crate::table::STATE_MASK;
#[cfg(target_arch = "x86_64")]
use crate::TransitionTable;
#[cfg(target_arch = "x86_64")]
use matchkit::Match;

type JitFn = unsafe extern "sysv64" fn(*const u8, u64, *mut Match, u64) -> u64;

/// Executable buffer backed by mmap'd memory (W^X: written as RW, flipped to RX).
#[cfg(target_arch = "x86_64")]
pub struct ExecutableBuffer {
    pub(crate) ptr: *mut u8,
    pub(crate) len: usize,
    pub(crate) table: Option<TransitionTable>,
    pub(crate) is_jit: bool,
    pub(crate) accept_pattern: Vec<u32>,
    pub(crate) output_links: Vec<u32>,
}

#[cfg(target_arch = "x86_64")]
impl std::fmt::Debug for ExecutableBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutableBuffer")
            .field("len", &self.len)
            .field("is_jit", &self.is_jit)
            .finish_non_exhaustive()
    }
}

// SAFETY: `ExecutableBuffer` owns the mmap region exclusively. The pointer is
// never mutated after construction and the buffer is not shared mutably.
#[cfg(target_arch = "x86_64")]
unsafe impl Send for ExecutableBuffer {}
// SAFETY: The underlying JIT code is immutable after finalization, so sharing
// the buffer across threads is safe as long as it is not mutated.
#[cfg(target_arch = "x86_64")]
unsafe impl Sync for ExecutableBuffer {}

#[cfg(target_arch = "x86_64")]
impl Drop for ExecutableBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.len > 0 {
            unsafe {
                libc::munmap(self.ptr.cast::<libc::c_void>(), self.len);
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
impl ExecutableBuffer {
    /// Scan input bytes, placing matches directly into the output slice.
    pub fn scan(&self, input: &[u8], matches: &mut [Match]) -> usize {
        if self.is_jit {
            self.scan_jit(input, matches)
        } else {
            self.scan_interpreted(input, matches)
        }
    }

    fn scan_jit(&self, input: &[u8], matches: &mut [Match]) -> usize {
        if input.is_empty() {
            return 0;
        }

        let max_matches = matches.len();
        let func: JitFn = unsafe { std::mem::transmute(self.ptr) };
        let count = unsafe {
            func(
                input.as_ptr(),
                input.len() as u64,
                matches.as_mut_ptr(),
                max_matches as u64,
            )
        };
        (count as usize).min(max_matches)
    }

    pub fn scan_count(&self, input: &[u8]) -> usize {
        if self.is_jit {
            self.scan_count_jit(input)
        } else {
            self.scan_count_interpreted(input)
        }
    }

    fn scan_count_jit(&self, input: &[u8]) -> usize {
        if input.is_empty() {
            return 0;
        }
        let func: JitFn = unsafe { std::mem::transmute(self.ptr) };
        let count = unsafe { func(input.as_ptr(), input.len() as u64, std::ptr::null_mut(), 0) };
        count as usize
    }

    fn scan_count_interpreted(&self, input: &[u8]) -> usize {
        let Some(table) = self.table.as_ref() else {
            return 0;
        };
        let mut state = 0u32;
        let mut count = 0usize;

        for &byte in input {
            let idx = state as usize * table.class_count() + byte as usize;
            let next = table.transitions().get(idx).copied().unwrap_or(0);
            let clean_next = next & STATE_MASK;

            if self
                .accept_pattern
                .get(clean_next as usize)
                .copied()
                .unwrap_or(0xFFFF_FFFF)
                == 0xFFFF_FFFF
            {
                state = clean_next;
            } else {
                let mut output_state = clean_next;
                while output_state != 0xFFFF_FFFF {
                    count += 1;
                    output_state = self
                        .output_links
                        .get(output_state as usize)
                        .copied()
                        .unwrap_or(0xFFFF_FFFF);
                }
                state = 0;
            }
        }
        count
    }

    fn scan_interpreted(&self, input: &[u8], matches: &mut [Match]) -> usize {
        let Some(table) = self.table.as_ref() else {
            return 0;
        };
        let mut state = 0u32;
        let mut count = 0usize;

        for (pos, &byte) in input.iter().enumerate() {
            let idx = state as usize * table.class_count() + byte as usize;
            let next = table.transitions().get(idx).copied().unwrap_or(0);
            let clean_next = next & STATE_MASK;

            if self
                .accept_pattern
                .get(clean_next as usize)
                .copied()
                .unwrap_or(0xFFFF_FFFF)
                == 0xFFFF_FFFF
            {
                state = clean_next;
            } else {
                let mut output_state = clean_next;
                while output_state != 0xFFFF_FFFF {
                    let pid = self
                        .accept_pattern
                        .get(output_state as usize)
                        .copied()
                        .unwrap_or(0);
                    if count < matches.len() {
                        let end = (pos + 1) as u32;
                        let pat_len = table
                            .pattern_lengths()
                            .get(pid as usize)
                            .copied()
                            .unwrap_or(0);
                        let start = end.saturating_sub(pat_len);
                        matches[count] = Match::from_parts(pid, start, end);
                    }
                    count += 1;
                    output_state = self
                        .output_links
                        .get(output_state as usize)
                        .copied()
                        .unwrap_or(0xFFFF_FFFF);
                }
                state = 0;
            }
        }
        count.min(matches.len())
    }
}
