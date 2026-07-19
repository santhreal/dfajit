use super::*;
use matchkit::Match;

fn new_table(state_count: usize, class_count: usize) -> TransitionTable {
    TransitionTable::new(state_count, class_count).unwrap()
}

fn simple_ab_table() -> TransitionTable {
    let mut table = new_table(3, 256);
    for state in 0..3 {
        for byte in 0..=255u8 {
            table.set_transition(state, byte, 0);
        }
        table.set_transition(state, b'a', 1);
    }
    table.set_transition(1, b'b', 2);
    table.add_accept(2, 0);
    table.set_pattern_length(0, 2);
    table
}

#[test]
fn compile_simple_dfa() {
    let table = simple_ab_table();
    let jit = JitDfa::compile(&table).unwrap();
    assert_eq!(jit.state_count(), 3);
    assert_eq!(jit.pattern_count(), 1);
}

#[test]
fn scan_finds_matches() {
    let table = simple_ab_table();
    let jit = JitDfa::compile(&table).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    let count = jit.scan(b"xabxab", &mut matches);
    assert_eq!(count, 2);
    assert_eq!(matches[0].start, 1);
    assert_eq!(matches[0].end, 3);
    assert_eq!(matches[1].start, 4);
    assert_eq!(matches[1].end, 6);
}

#[test]
fn scan_empty_input() {
    let table = simple_ab_table();
    let jit = JitDfa::compile(&table).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    assert_eq!(jit.scan(b"", &mut matches), 0);
}

#[test]
fn scan_count_matches_scan_len() {
    let table = simple_ab_table();
    let jit = JitDfa::compile(&table).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    assert_eq!(jit.scan_count(b"xabxab"), jit.scan(b"xabxab", &mut matches));
}

#[test]
fn scan_no_match() {
    let table = simple_ab_table();
    let jit = JitDfa::compile(&table).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    assert_eq!(jit.scan(b"xxxxxx", &mut matches), 0);
}

#[test]
fn scan_consecutive_matches() {
    let table = simple_ab_table();
    let jit = JitDfa::compile(&table).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    assert_eq!(jit.scan(b"ababab", &mut matches), 3);
}

#[test]
fn scan_first_returns_first_match_only() {
    let table = simple_ab_table();
    let jit = JitDfa::compile(&table).unwrap();
    let first = jit.scan_first(b"zzabzzab").unwrap();
    assert_eq!(first.start, 2);
    assert_eq!(first.end, 4);
}

#[test]
fn has_match_reports_presence() {
    let table = simple_ab_table();
    let jit = JitDfa::compile(&table).unwrap();
    assert!(jit.has_match(b"xab"));
    assert!(!jit.has_match(b"zzz"));
}

#[test]
fn empty_dfa_rejected() {
    assert!(JitDfa::compile(&new_table(0, 256)).is_err());
}

#[test]
fn invalid_table_size_rejected() {
    let mut table = new_table(3, 256);
    table.transitions_raw_mut().truncate(10);
    assert!(JitDfa::compile(&table).is_err());
}

#[test]
fn transitions_mut_allows_element_edit_but_preserves_size_invariant() {
    let mut table = new_table(2, 256);
    let expected_len = 2 * 256;
    assert_eq!(table.transitions_mut().len(), expected_len);

    // Element mutation through the slice accessor is allowed and reflected.
    table.transitions_mut()[5] = 1;
    assert_eq!(table.transitions()[5], 1);

    // The length invariant is UNCHANGED: the slice accessor structurally cannot
    // push/truncate (compile-time guarantee), so a valid table stays valid and
    // still compiles. Length corruption is only reachable via the doc-hidden
    // transitions_raw_mut, exercised by invalid_table_size_rejected above.
    assert_eq!(table.transitions().len(), expected_len);
    assert!(JitDfa::compile(&table).is_ok());
}

#[test]
fn multi_pattern_dfa() {
    let mut table = new_table(3, 256);
    for byte in 0..=255u8 {
        table.set_transition(0, byte, 0);
        table.set_transition(1, byte, 0);
        table.set_transition(2, byte, 0);
    }
    table.set_transition(0, b'a', 1);
    table.set_transition(0, b'b', 2);
    table.set_transition(1, b'a', 1);
    table.set_transition(1, b'b', 2);
    table.set_transition(2, b'a', 1);
    table.set_transition(2, b'b', 2);
    table.add_accept(1, 0);
    table.add_accept(2, 1);
    table.set_pattern_length(0, 1);
    table.set_pattern_length(1, 1);

    let jit = JitDfa::compile(&table).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    let count = jit.scan(b"ab", &mut matches);
    assert_eq!(count, 2);
    assert_eq!(matches[0].pattern_id, 0);
    assert_eq!(matches[1].pattern_id, 1);
}

#[test]
fn from_patterns_builds_literal_matcher() {
    let jit = JitDfa::from_patterns(&[b"foo", b"bar"]).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    let count = jit.scan(b"foo bar", &mut matches);
    assert_eq!(count, 2);
    assert_eq!(matches[0].end, 3);
    assert_eq!(matches[1].end, 7);
}

#[test]
fn from_patterns_rejects_empty_pattern_set() {
    assert!(JitDfa::from_patterns(&[]).is_err());
}

#[test]
fn from_patterns_ignores_empty_literals() {
    let jit = JitDfa::from_patterns(&[b"", b"x"]).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    assert_eq!(jit.scan(b"x", &mut matches), 1);
    assert_eq!(matches[0].pattern_id, 1);
}

#[test]
fn serialization_round_trip() {
    let table = simple_ab_table();
    let bytes = table.to_bytes();
    let restored = TransitionTable::from_bytes(&bytes).unwrap();
    assert_eq!(restored.state_count(), table.state_count());
    assert_eq!(restored.class_count(), table.class_count());
    assert_eq!(restored.transitions(), table.transitions());
    assert_eq!(restored.accept_states(), table.accept_states());
    assert_eq!(restored.pattern_lengths(), table.pattern_lengths());

    let jit = JitDfa::compile(&restored).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    assert_eq!(jit.scan(b"xabxab", &mut matches), 2);
}

#[test]
fn serialization_rejects_truncated() {
    let table = simple_ab_table();
    let bytes = table.to_bytes();
    assert!(TransitionTable::from_bytes(&bytes[..10]).is_err());
}

#[test]
fn serialization_rejects_truncated_accept_metadata() {
    let table = simple_ab_table();
    let mut bytes = table.to_bytes();
    bytes.truncate(bytes.len() - 6);
    assert!(TransitionTable::from_bytes(&bytes).is_err());
}

#[test]
fn serialization_rejects_truncated_pattern_lengths() {
    let table = simple_ab_table();
    let mut bytes = table.to_bytes();
    bytes.truncate(bytes.len() - 2);
    assert!(TransitionTable::from_bytes(&bytes).is_err());
}
