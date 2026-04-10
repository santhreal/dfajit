#![allow(clippy::panic, clippy::unwrap_used)]

use dfajit::{JitDfa, TransitionTable};
use matchkit::Match;

#[test]
fn dfa_with_256_states_max_byte_transitions() {
    let mut table = TransitionTable::new(256, 256).expect("table");
    for byte in 0_u16..=255 {
        table.set_transition(0, byte as u8, byte as u32);
    }
    for state in 1_u32..=255 {
        table.add_accept(state, state);
        table.set_pattern_length(state, 1);
    }

    let jit = JitDfa::compile(&table).expect("compile");
    let input: Vec<u8> = (1_u16..=255).map(|b| b as u8).collect();
    let mut matches = vec![Match::from_parts(0, 0, 0); input.len()];
    let count = jit.scan(&input, &mut matches);

    assert_eq!(count, input.len(), "256-state DFA lost transitions or matches");
    assert_eq!(matches[0].pattern_id, 1);
    assert_eq!(matches[input.len() - 1].pattern_id, 255);
}

#[test]
fn accept_state_on_every_transition_reports_total_matches() {
    let mut table = TransitionTable::new(1, 256).expect("table");
    for byte in 0_u16..=255 {
        table.set_transition(0, byte as u8, 0);
    }
    table.add_accept(0, 0);
    table.set_pattern_length(0, 1);

    let jit = JitDfa::compile(&table).expect("compile");
    let input = vec![b'x'; 64];
    let mut matches = vec![Match::from_parts(0, 0, 0); 8];

    assert_eq!(jit.scan_count(&input), 64, "scan_count should see every accept transition");
    assert_eq!(jit.scan(&input, &mut matches), 64, "scan should report total matches even when the output buffer is smaller");
}

#[test]
fn empty_input_returns_no_match() {
    let jit = JitDfa::from_patterns(&[b"abc"]).expect("compile literal DFA");
    let mut matches = vec![Match::from_parts(0, 0, 0); 4];

    assert_eq!(jit.scan(b"", &mut matches), 0, "empty input should produce zero matches");
    assert_eq!(jit.scan_count(b""), 0, "empty input should count as zero matches");
    assert!(jit.scan_first(b"").is_none(), "empty input should not have a first match");
}

#[test]
fn input_100mb_with_sparse_markers_counts_every_match() {
    let jit = JitDfa::from_patterns(&[b"Z"]).expect("compile literal DFA");
    let mut input = vec![b'a'; 100 * 1024 * 1024];
    let expected = 100;
    for i in 0..expected {
        input[i * 1024 * 1024] = b'Z';
    }

    assert_eq!(jit.scan_count(&input), expected, "100MB sparse scan lost matches");
}
