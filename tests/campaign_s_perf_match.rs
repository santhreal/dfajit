//! S-perf-match campaign catalog — +40 dfajit DFA/JIT smoke cases.

#![allow(clippy::unwrap_used)]

use dfajit::{JitDfa, TransitionTable};
use matchkit::Match;

fn reset_table(state_count: usize) -> TransitionTable {
    let mut table = TransitionTable::new(state_count, 256).unwrap();
    for state in 0..state_count {
        for byte in 0..=255u8 {
            table.set_transition(state, byte, 0);
        }
    }
    table
}

fn ab_jit() -> JitDfa {
    let mut table = reset_table(3);
    table.set_transition(0, b'a', 1);
    table.set_transition(1, b'b', 2);
    table.add_accept(2, 0);
    table.set_pattern_length(0, 2);
    JitDfa::compile(&table).unwrap()
}

macro_rules! campaign_scan {
    ($name:ident, $input:expr, $expect:expr) => {
        #[test]
        fn $name() {
            let jit = ab_jit();
            let mut matches = vec![Match::from_parts(0, 0, 0); 8];
            let count = jit.scan($input, &mut matches);
            assert_eq!(count, $expect);
        }
    };
}

campaign_scan!(c00, b"", 0);
campaign_scan!(c01, b"ab", 1);
campaign_scan!(c02, b"xab", 1);
campaign_scan!(c03, b"abab", 2);
campaign_scan!(c04, b"abxab", 2);

#[test]
fn campaign_scan_no_match() {
    let jit = ab_jit();
    let mut matches = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"xy", &mut matches), 0);
}

#[test]
fn campaign_scan_count_matches_scan() {
    let jit = ab_jit();
    assert_eq!(jit.scan_count(b"abab"), 2);
    let mut buf = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"abab", &mut buf), 2);
}

#[test]
fn campaign_scan_first_finds_one() {
    let jit = ab_jit();
    let m = jit.scan_first(b"xxabxxab").expect("first match");
    assert_eq!(m.start, 2);
}

#[test]
fn campaign_compile_single_state() {
    let table = reset_table(1);
    assert!(JitDfa::compile(&table).is_ok());
}

#[test]
fn campaign_compile_rejects_zero_states() {
    let table = TransitionTable::new(0, 256).expect("empty table is constructible");
    assert!(JitDfa::compile(&table).is_err());
}

#[test]
fn campaign_compile_rejects_too_many_states() {
    assert!(TransitionTable::new(TransitionTable::MAX_STATES + 1, 256).is_err());
}

#[test]
fn campaign_single_byte_pattern() {
    let mut table = reset_table(2);
    table.set_transition(0, b'x', 1);
    table.add_accept(1, 0);
    table.set_pattern_length(0, 1);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"xxx", &mut m), 3);
}

#[test]
fn campaign_loop_back_to_start() {
    let mut table = reset_table(2);
    table.set_transition(0, b'a', 1);
    table.set_transition(1, b'a', 0);
    table.add_accept(1, 0);
    table.set_pattern_length(0, 2);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    // Overlapping accept at both positions for `aa` on loop-back DFA.
    assert_eq!(jit.scan(b"aa", &mut m), 2);
}

#[test]
fn campaign_two_patterns() {
    let mut table = reset_table(3);
    table.set_transition(0, b'a', 1);
    table.set_transition(0, b'b', 2);
    table.add_accept(1, 0);
    table.add_accept(2, 1);
    table.set_pattern_length(0, 1);
    table.set_pattern_length(1, 1);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 8];
    assert_eq!(jit.scan(b"ab", &mut m), 2);
}

#[test]
fn campaign_dead_state_stays_zero() {
    let table = reset_table(4);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"anything", &mut m), 0);
}

#[test]
fn campaign_accept_at_start_state() {
    let mut table = reset_table(1);
    table.add_accept(0, 0);
    table.set_pattern_length(0, 0);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    // Empty input: no byte positions to report accept (state-0 accept is compile-valid only).
    assert_eq!(jit.scan(b"", &mut m), 0);
}

#[test]
fn campaign_long_input_no_panic() {
    let jit = ab_jit();
    let input = vec![b'a'; 10_000];
    let mut m = vec![Match::from_parts(0, 0, 0); 16];
    let _ = jit.scan(&input, &mut m);
}

#[test]
fn campaign_all_bytes_transition() {
    let mut table = reset_table(2);
    for byte in 0..=255u8 {
        table.set_transition(0, byte, 1);
    }
    table.add_accept(1, 0);
    table.set_pattern_length(0, 1);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(&[0u8], &mut m), 1);
}

#[test]
fn campaign_pattern_length_stored() {
    let mut table = reset_table(2);
    table.set_transition(0, b'z', 1);
    table.add_accept(1, 0);
    table.set_pattern_length(0, 7);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    let count = jit.scan(b"z", &mut m);
    assert_eq!(count, 1);
    // Reported span width follows `pattern_length`, not consumed bytes.
    assert_eq!(m[0].end.saturating_sub(m[0].start), 1);
    assert_eq!(table.pattern_lengths()[0], 7);
}

#[test]
fn campaign_three_state_chain() {
    let mut table = reset_table(4);
    table.set_transition(0, b'a', 1);
    table.set_transition(1, b'b', 2);
    table.set_transition(2, b'c', 3);
    table.add_accept(3, 0);
    table.set_pattern_length(0, 3);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"abc", &mut m), 1);
}

#[test]
fn campaign_scan_buffer_smaller_than_matches() {
    let jit = ab_jit();
    let mut m = vec![Match::from_parts(0, 0, 0); 1];
    let count = jit.scan(b"abab", &mut m);
    assert!(count <= 1);
}

#[test]
fn campaign_compile_max_states_ok() {
    let table = reset_table(4096);
    assert!(JitDfa::compile(&table).is_ok());
}

#[test]
fn campaign_self_loop_on_state() {
    let mut table = reset_table(2);
    table.set_transition(0, b'x', 0);
    table.set_transition(0, b'y', 1);
    table.add_accept(1, 0);
    table.set_pattern_length(0, 1);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"y", &mut m), 1);
}

#[test]
fn campaign_binary_input() {
    let jit = ab_jit();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(&[0, 97, 98, 255], &mut m), 1);
}

#[test]
fn campaign_scan_count_empty() {
    let jit = ab_jit();
    assert_eq!(jit.scan_count(b""), 0);
}

#[test]
fn campaign_multiple_accept_same_state() {
    let mut table = reset_table(3);
    table.set_transition(0, b'm', 1);
    table.set_transition(1, b'n', 2);
    table.add_accept(1, 0);
    table.set_pattern_length(0, 1);
    table.add_accept(2, 1);
    table.set_pattern_length(1, 1);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 8];
    let count = jit.scan(b"m", &mut m);
    assert!(count >= 1);
}

#[test]
fn campaign_transition_table_class_count() {
    let table = TransitionTable::new(8, 256).unwrap();
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 2];
    assert_eq!(jit.scan(b"\x00", &mut m), 0);
}

#[test]
fn campaign_ab_prefix_only() {
    let jit = ab_jit();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"a", &mut m), 0);
}

#[test]
fn campaign_repeated_ab_stream() {
    let jit = ab_jit();
    let input = b"ab".repeat(50);
    assert_eq!(jit.scan_count(&input), 50);
}

#[test]
fn campaign_case_sensitive_bytes() {
    let mut table = reset_table(2);
    table.set_transition(0, b'A', 1);
    table.add_accept(1, 0);
    table.set_pattern_length(0, 1);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"A", &mut m), 1);
    assert_eq!(jit.scan(b"a", &mut m), 0);
}

#[test]
fn campaign_high_state_index_transition() {
    let mut table = reset_table(100);
    table.set_transition(0, b'q', 99);
    table.add_accept(99, 0);
    table.set_pattern_length(0, 1);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"q", &mut m), 1);
}

#[test]
fn campaign_scan_first_none() {
    let jit = ab_jit();
    assert!(jit.scan_first(b"zzzz").is_none());
}

#[test]
fn campaign_compile_deterministic() {
    let t = reset_table(5);
    let a = JitDfa::compile(&t).unwrap();
    let b = JitDfa::compile(&t).unwrap();
    let mut ma = vec![Match::from_parts(0, 0, 0); 2];
    let mut mb = vec![Match::from_parts(0, 0, 0); 2];
    assert_eq!(
        a.scan(b"probe", &mut ma),
        b.scan(b"probe", &mut mb)
    );
}

#[test]
fn campaign_utf8_bytes_as_input() {
    let jit = ab_jit();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    let _ = jit.scan("αβ".as_bytes(), &mut m);
}

#[test]
fn campaign_newline_in_input() {
    let jit = ab_jit();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    assert_eq!(jit.scan(b"\nab\n", &mut m), 1);
}

#[test]
fn campaign_null_byte_in_input() {
    let jit = ab_jit();
    let mut m = vec![Match::from_parts(0, 0, 0); 4];
    let _ = jit.scan(b"\x00ab\x00", &mut m);
}

#[test]
fn campaign_two_accept_patterns_lengths() {
    let mut table = reset_table(3);
    table.set_transition(0, b'1', 1);
    table.set_transition(0, b'2', 2);
    table.add_accept(1, 0);
    table.add_accept(2, 1);
    table.set_pattern_length(0, 1);
    table.set_pattern_length(1, 2);
    let jit = JitDfa::compile(&table).unwrap();
    let mut m = vec![Match::from_parts(0, 0, 0); 8];
    assert_eq!(jit.scan(b"12", &mut m), 2);
}
