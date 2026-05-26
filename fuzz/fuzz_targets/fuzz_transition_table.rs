#![no_main]
use libfuzzer_sys::fuzz_target;
use dfajit::{JitDfa, TransitionTable};

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let state_count = (u16::from_le_bytes([data[0], data[1]]) as usize % 64).max(1);
    let Ok(mut table) = TransitionTable::new(state_count, 256) else {
        return;
    };
    let mut i = 2usize;
    while i + 2 < data.len() {
        let from = (data[i] as usize) % state_count;
        let byte = data[i + 1];
        let to = (data[i + 2] as usize % state_count) as u32;
        table.set_transition(from, byte, to);
        i += 3;
    }
    if i < data.len() {
        let accept = (data[i] as usize) % state_count;
        table.add_accept(accept as u32, 0);
        table.set_pattern_length(0, 1);
    }
    if let Ok(jit) = JitDfa::compile(&table) {
        let mut matches = vec![matchkit::Match::from_parts(0, 0, 0); 16];
        let _ = jit.scan(&data[i.min(data.len())..], &mut matches);
    }
});
