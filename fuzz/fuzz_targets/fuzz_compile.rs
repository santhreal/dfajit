#![no_main]
use libfuzzer_sys::fuzz_target;
use dfajit::TransitionTable;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let state_count = u16::from_le_bytes([data[0], data[1]]) as usize;
    let state_count = state_count.clamp(1, 256);
    let Ok(mut table) = TransitionTable::new(state_count, 256) else {
        return;
    };

    let mut i = 2;
    while i + 3 < data.len() {
        let from_state = data[i] as u32 % state_count as u32;
        let byte = data[i + 1];
        let to_state = data[i + 2] as u32 % state_count as u32;
        table.set_transition(from_state as usize, byte, to_state);
        i += 3;
    }

    if i + 1 < data.len() {
        let accept_state = data[i] as u32 % state_count as u32;
        let pattern_id = data[i + 1] as u32 % 16;
        table.add_accept(accept_state, pattern_id);
        table.set_pattern_length(pattern_id, 1);
    }

    let _ = dfajit::JitDfa::compile(&table);
});
