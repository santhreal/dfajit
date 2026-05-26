#![no_main]
use libfuzzer_sys::fuzz_target;
use dfajit::{JitDfa, TransitionTable};

fuzz_target!(|data: &[u8]| {
    let Ok(mut table) = TransitionTable::new(3, 256) else {
        return;
    };
    for state in 0..3 {
        for byte in 0..=255u8 {
            table.set_transition(state, byte, 0);
        }
        table.set_transition(state, b'a', 1);
    }
    table.set_transition(1, b'b', 2);
    table.add_accept(2, 0);
    table.set_pattern_length(0, 2);

    let Ok(jit) = JitDfa::compile(&table) else {
        return;
    };

    let mut matches = vec![matchkit::Match::from_parts(0, 0, 0); 32];
    let _ = jit.scan(data, &mut matches);
});
