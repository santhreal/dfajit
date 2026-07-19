//! x86_64 JIT emission for DFA transition tables.

#[cfg(target_arch = "x86_64")]
use crate::error::{Error, Result};
#[cfg(target_arch = "x86_64")]
use crate::TransitionTable;

#[cfg(target_arch = "x86_64")]
use super::buffer::ExecutableBuffer;

/// Maximum states for JIT (I-cache ≈ 32KB).
#[cfg(target_arch = "x86_64")]
const MAX_JIT_STATES: usize = 4096;

/// Compile a DFA transition table to native x86_64 machine code.
#[cfg(target_arch = "x86_64")]
pub fn compile_x86_64(table: &TransitionTable, output_links: &[u32]) -> Result<ExecutableBuffer> {
    if table.state_count() > 65_536 {
        return Err(Error::TooManyStates {
            states: table.state_count(),
            max: 65_536,
        });
    }
    if table.state_count() > MAX_JIT_STATES {
        return compile_interpreted_fallback(table, output_links);
    }

    let mut flagged = table.transitions().to_vec();
    let mut accept_pattern: Vec<u32> = vec![0xFFFF_FFFF; table.state_count()];
    for &(state, pattern_id) in table.accept_states() {
        if (state as usize) < accept_pattern.len() {
            accept_pattern[state as usize] = pattern_id;
        }
    }
    for t in &mut flagged {
        let target = (*t & 0x7FFF_FFFF) as usize;
        if target < accept_pattern.len() && accept_pattern[target] != 0xFFFF_FFFF {
            *t = target as u32 | 0x8000_0000;
        } else {
            *t = target as u32;
        }
    }


    let mut output_link = output_links.to_vec();
    if output_link.len() < table.state_count() {
        output_link.resize(table.state_count(), 0xFFFF_FFFF);
    }

    let mut c: Vec<u8> = Vec::with_capacity(4096);
    c.extend_from_slice(&[0x53, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57]);
    c.extend_from_slice(&[
        0x49, 0x89, 0xFC, 0x48, 0x89, 0xF5, 0x49, 0x89, 0xD6, 0x48, 0x89, 0xCB,
    ]);
    c.extend_from_slice(&[0x45, 0x31, 0xED, 0x45, 0x31, 0xFF, 0x45, 0x31, 0xDB]);
    c.extend_from_slice(&[0x49, 0x39, 0xED, 0x0F, 0x83]);
    let empty_patch = c.len();
    c.extend_from_slice(&[0; 4]);

    // Load the transition table base once into the callee-saved r10 register
    // before entering the hot scan loop, instead of re-emitting a 10-byte
    // `movabs rdi, <trans_base>` on every input byte.
    let trans_patch = c.len();
    c.extend_from_slice(&[0x49, 0xBA]);
    c.extend_from_slice(&[0; 8]);

    let scan_top = c.len();
    c.extend_from_slice(&[0x43, 0x0F, 0xB6, 0x04, 0x2C]);
    c.extend_from_slice(&[0x41, 0x69, 0xD3]);
    c.extend_from_slice(&(table.class_count() as u32).to_le_bytes());
    c.extend_from_slice(&[0x01, 0xC2]);

    // Transition lookup uses the pre-loaded r10 base; rdx is the byte index.
    c.extend_from_slice(&[0x41, 0x8B, 0x04, 0x92, 0x89, 0xC1, 0x25]);
    c.extend_from_slice(&0x7FFF_FFFFu32.to_le_bytes());
    c.extend_from_slice(&[0x41, 0x89, 0xC3, 0xF7, 0xC1]);
    c.extend_from_slice(&0x8000_0000u32.to_le_bytes());
    c.extend_from_slice(&[0x0F, 0x84]);
    let skip_match_patch = c.len();
    c.extend_from_slice(&[0; 4]);

    c.extend_from_slice(&[0x45, 0x89, 0xD8]);
    let accept_loop = c.len();

    c.extend_from_slice(&[0x49, 0x39, 0xDF, 0x0F, 0x83]);
    let skip_write_match_patch = c.len();
    c.extend_from_slice(&[0; 4]);

    let accept_patch = c.len();
    c.extend_from_slice(&[0x48, 0xBF]);
    c.extend_from_slice(&[0; 8]);
    c.extend_from_slice(&[
        0x42, 0x8B, 0x04, 0x87, 0x4C, 0x89, 0xFF, 0x48, 0x6B, 0xFF, 0x0C, 0x4C, 0x01, 0xF7, 0x89,
        0x07, 0x89, 0xC1,
    ]);

    let patlen_patch = c.len();
    c.extend_from_slice(&[0x48, 0xBA]);
    c.extend_from_slice(&[0; 8]);
    c.extend_from_slice(&[
        0x8B, 0x0C, 0x8A, 0x44, 0x89, 0xEA, 0x83, 0xC2, 0x01, 0x89, 0x57, 0x08, 0x29, 0xCA, 0x73,
        0x02, 0x31, 0xD2, 0x89, 0x57, 0x04,
    ]);

    let skip_write_match_target = c.len();
    patch_rel32(&mut c, skip_write_match_patch, skip_write_match_target);
    c.extend_from_slice(&[0x49, 0xFF, 0xC7]);

    let output_patch = c.len();
    c.extend_from_slice(&[0x48, 0xBF]);
    c.extend_from_slice(&[0; 8]);
    c.extend_from_slice(&[0x46, 0x8B, 0x04, 0x87, 0x41, 0x81, 0xF8]);
    c.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    c.extend_from_slice(&[0x0F, 0x85]);
    let accept_loop_patch = c.len();
    c.extend_from_slice(&[0; 4]);
    patch_rel32(&mut c, accept_loop_patch, accept_loop);

    c.extend_from_slice(&[0x45, 0x31, 0xDB]);
    let skip_match_target = c.len();
    patch_rel32(&mut c, skip_match_patch, skip_match_target);
    c.extend_from_slice(&[
        0x49, 0xFF, 0xC5, 0x43, 0x0F, 0x18, 0x44, 0x2C, 0x40, 0x49, 0x39, 0xED, 0x0F, 0x82,
    ]);
    let loop_patch = c.len();
    c.extend_from_slice(&[0; 4]);
    patch_rel32(&mut c, loop_patch, scan_top);

    let epilogue = c.len();
    patch_rel32(&mut c, empty_patch, epilogue);
    c.extend_from_slice(&[
        0x4C, 0x89, 0xF8, 0x41, 0x5F, 0x41, 0x5E, 0x41, 0x5D, 0x41, 0x5C, 0x5D, 0x5B, 0xC3,
    ]);

    while c.len() % 8 != 0 {
        c.push(0xCC);
    }

    let trans_offset = c.len();
    for &t in &flagged {
        c.extend_from_slice(&t.to_le_bytes());
    }

    let accept_offset = c.len();
    for &p in &accept_pattern {
        c.extend_from_slice(&p.to_le_bytes());
    }

    let patlen_offset = c.len();
    if table.pattern_lengths().is_empty() {
        c.extend_from_slice(&0u32.to_le_bytes());
    } else {
        for &l in table.pattern_lengths() {
            c.extend_from_slice(&l.to_le_bytes());
        }
    }

    let output_offset = c.len();
    for &o in &output_link {
        c.extend_from_slice(&o.to_le_bytes());
    }

    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }
        .try_into()
        .unwrap_or(4096usize)
        .max(4096);
    let alloc_size = (c.len() + page_size - 1) & !(page_size - 1);

    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            alloc_size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        return Err(Error::MemoryAllocation {
            reason: format!(
                "mmap(RW, {alloc_size}) failed: {}",
                std::io::Error::last_os_error()
            ),
        });
    }

    let buf = ptr.cast::<u8>();
    unsafe {
        std::ptr::copy_nonoverlapping(c.as_ptr(), buf, c.len());
    }

    let base = buf as u64;
    patch_imm64(&mut c, buf, trans_patch + 2, base + trans_offset as u64);
    patch_imm64(&mut c, buf, accept_patch + 2, base + accept_offset as u64);
    patch_imm64(&mut c, buf, patlen_patch + 2, base + patlen_offset as u64);
    patch_imm64(&mut c, buf, output_patch + 2, base + output_offset as u64);

    let prot = unsafe { libc::mprotect(ptr, alloc_size, libc::PROT_READ | libc::PROT_EXEC) };
    if prot != 0 {
        unsafe {
            libc::munmap(ptr, alloc_size);
        }
        return Err(Error::MemoryAllocation {
            reason: format!("mprotect(RX) failed: {}", std::io::Error::last_os_error()),
        });
    }

    Ok(ExecutableBuffer {
        ptr: buf,
        len: alloc_size,
        table: None,
        is_jit: true,
        accept_pattern,
        output_links: output_link,
    })
}

#[cfg(target_arch = "x86_64")]
fn patch_rel32(code: &mut [u8], site: usize, target: usize) {
    let rel = target as isize - (site + 4) as isize;
    debug_assert!(
        i32::try_from(rel).is_ok(),
        "Fix: JIT code size exceeded 2GB, which should be impossible with MAX_JIT_STATES=4096."
    );
    let rel = i32::try_from(rel).unwrap_or(0);
    code[site..site + 4].copy_from_slice(&rel.to_le_bytes());
}

#[cfg(target_arch = "x86_64")]
fn patch_imm64(code: &mut [u8], buf: *mut u8, offset: usize, value: u64) {
    let bytes = value.to_le_bytes();
    code[offset..offset + 8].copy_from_slice(&bytes);
    // SAFETY: `buf` points to an mmap'd region of at least `code.len()` bytes,
    // and `offset + 8` is within the code section we already copied.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf.add(offset), 8);
    }
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn compile_interpreted_fallback(
    table: &TransitionTable,
    output_links: &[u32],
) -> Result<ExecutableBuffer> {
    // The interpreted path (`is_jit == false`) scans in software via `table`,
    // `accept_pattern`, and `output_links`; it never transmutes or calls
    // `ptr`. The previous code mmap'd a page, wrote a single `RET` (0xC3) into
    // it, flipped it to PROT_EXEC, and later munmap'd it - two syscalls plus an
    // unused executable page allocated on EVERY interpreted compile (which is
    // the default path, see dfa.rs). Use a null pointer / zero length instead;
    // `ExecutableBuffer`'s Drop already skips `munmap` when `ptr` is null or
    // `len` is 0, and no scan path dereferences `ptr` when `is_jit` is false.
    let mut accept_pattern = vec![0xFFFF_FFFF; table.state_count()];
    for &(state, pid) in table.accept_states() {
        if (state as usize) < accept_pattern.len() {
            accept_pattern[state as usize] = pid;
        }
    }

    let mut output_link = output_links.to_vec();
    if output_link.len() < table.state_count() {
        output_link.resize(table.state_count(), 0xFFFF_FFFF);
    }

    Ok(ExecutableBuffer {
        ptr: std::ptr::null_mut(),
        len: 0,
        table: Some(table.clone()),
        is_jit: false,
        accept_pattern,
        output_links: output_link,
    })
}
