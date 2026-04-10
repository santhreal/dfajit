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
fn is_jit_eligible_small() {
    assert!(simple_ab_table().is_jit_eligible());
}

#[test]
fn estimated_code_size() {
    let size = simple_ab_table().estimated_code_size();
    assert!(size > 0);
    assert!(size < 100_000);
}

#[test]
fn large_dfa_is_not_jit_eligible() {
    assert!(!new_table(4097, 256).is_jit_eligible());
}

#[test]
fn minimize_reduces_redundant_states() {
    let mut table = new_table(4, 256);
    for s in 0..4 {
        for b in 0..=255u8 {
            table.set_transition(s, b, 0);
        }
    }
    table.set_transition(0, b'x', 3);
    table.set_transition(1, b'x', 3);
    table.set_transition(2, b'x', 3);
    table.add_accept(3, 0);
    table.set_pattern_length(0, 1);

    if let Some(minimized) = table.minimize() {
        assert!(minimized.state_count() < table.state_count());
        let jit = JitDfa::compile(&minimized).unwrap();
        let mut matches = vec![Match::from_parts(0, 0, 0); 10];
        assert_eq!(jit.scan(b"x", &mut matches), 1);
    }
}

#[test]
fn minimize_already_minimal() {
    let table = simple_ab_table();
    if let Some(minimized) = table.minimize() {
        assert!(minimized.state_count() <= table.state_count());
    }
}

#[test]
fn minimize_preserves_behavior() {
    let mut table = new_table(5, 256);
    for s in 0..5 {
        for b in 0..=255u8 {
            table.set_transition(s, b, 0);
        }
        table.set_transition(s, b'a', 1);
    }
    table.set_transition(1, b'b', 2);
    table.set_transition(2, b'c', 3);
    table.add_accept(3, 0);
    table.set_pattern_length(0, 3);

    let original = JitDfa::compile(&table).unwrap();
    let input = b"xabcxabc";
    let mut orig_matches = vec![Match::from_parts(0, 0, 0); 10];
    let orig_count = original.scan(input, &mut orig_matches);

    if let Some(minimized) = table.minimize() {
        let min_jit = JitDfa::compile(&minimized).unwrap();
        let mut min_matches = vec![Match::from_parts(0, 0, 0); 10];
        assert_eq!(orig_count, min_jit.scan(input, &mut min_matches));
    }
}

#[test]
fn compute_ranges_collapses_consecutive_targets() {
    let mut table = new_table(2, 256);
    for byte in 0..=u8::MAX {
        table.set_transition(0, byte, 0);
    }
    for byte in b'a'..=b'z' {
        table.set_transition(0, byte, 1);
    }
    assert_eq!(table.compute_ranges()[0], vec![(0, 96, 0), (97, 122, 1), (123, 255, 0)]);
}

#[test]
fn compute_ranges_finds_expected_character_classes() {
    let mut table = new_table(3, 256);
    for byte in b'a'..=b'z' {
        table.set_transition(0, byte, 1);
    }
    for byte in b'A'..=b'Z' {
        table.set_transition(0, byte, 1);
    }
    for byte in b'0'..=b'9' {
        table.set_transition(0, byte, 2);
    }

    let interesting: Vec<_> = table
        .compute_ranges()
        .remove(0)
        .into_iter()
        .filter(|(_, _, target)| *target != 0)
        .collect();
    assert_eq!(interesting, vec![(b'0', b'9', 2), (b'A', b'Z', 1), (b'a', b'z', 1)]);
}

#[cfg(feature = "regex")]
#[test]
fn from_regex_patterns_finds_all_literals() {
    let jit = JitDfa::from_regex_patterns(&["hello", "world"]).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    let count = jit.scan(b"say hello to the world", &mut matches);
    assert_eq!(count, 2);
    assert_eq!(matches[0].start, 4);
    assert_eq!(matches[0].end, 9);
    assert_eq!(matches[1].start, 17);
    assert_eq!(matches[1].end, 22);
}

#[test]
fn compute_ranges_detects_alpha_ranges() {
    let mut table = new_table(3, 256);
    for b in b'a'..=b'z' {
        table.set_transition(0, b, 1);
    }
    for b in b'A'..=b'Z' {
        table.set_transition(0, b, 1);
    }
    for b in b'0'..=b'9' {
        table.set_transition(0, b, 2);
    }

    let state0 = table.compute_ranges().remove(0);
    assert_eq!(state0.len(), 7);
    assert_eq!(state0.iter().find(|r| r.0 == b'a').unwrap(), &(b'a', b'z', 1));
    assert_eq!(state0.iter().find(|r| r.0 == b'A').unwrap(), &(b'A', b'Z', 1));
    assert_eq!(state0.iter().find(|r| r.0 == b'0').unwrap(), &(b'0', b'9', 2));
}

#[test]
fn compute_ranges_all_same_target() {
    assert_eq!(new_table(2, 256).compute_ranges()[0], vec![(0, 255, 0)]);
}

#[test]
fn compute_ranges_every_byte_different() {
    let mut table = new_table(257, 256);
    for b in 0u16..256 {
        table.set_transition(0, u8::try_from(b).unwrap_or(0), u32::from(b) + 1);
    }
    assert_eq!(table.compute_ranges()[0].len(), 256);
}

#[test]
fn transition_density_single_target() {
    assert_eq!(new_table(2, 256).transition_density(0), 1);
}

#[test]
fn transition_density_alpha_numeric() {
    let mut table = new_table(3, 256);
    for b in b'a'..=b'z' {
        table.set_transition(0, b, 1);
    }
    for b in b'0'..=b'9' {
        table.set_transition(0, b, 2);
    }
    assert_eq!(table.transition_density(0), 3);
}

#[test]
fn minimize_preserves_different_pattern_ids() {
    let mut table = new_table(3, 256);
    for b in 0..=255u8 {
        table.set_transition(0, b, 0);
        table.set_transition(1, b, 0);
        table.set_transition(2, b, 0);
    }
    table.set_transition(0, b'a', 1);
    table.set_transition(0, b'b', 2);
    table.add_accept(1, 0);
    table.add_accept(2, 1);
    table.set_pattern_length(0, 1);
    table.set_pattern_length(1, 1);

    let minimized = table.minimize().unwrap_or(table.clone());
    let jit = JitDfa::compile(&minimized).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    let count = jit.scan(b"ab", &mut matches);
    assert_eq!(count, 2);
    assert_eq!(matches[0].pattern_id, 0);
    assert_eq!(matches[1].pattern_id, 1);
}

#[test]
fn minimize_all_accept_same_pattern() {
    let mut table = new_table(2, 256);
    for b in 0..=255u8 {
        table.set_transition(0, b, 0);
        table.set_transition(1, b, 1);
    }
    table.add_accept(0, 0);
    table.add_accept(1, 0);
    table.set_pattern_length(0, 1);

    let minimized = table.minimize().expect("should minimize");
    assert_eq!(minimized.state_count(), 1);
    assert_eq!(JitDfa::compile(&minimized).unwrap().scan_count(b"xxx"), 3);
}

#[test]
fn minimize_all_dead_collapses_to_one() {
    let mut table = new_table(3, 256);
    for state in 0..3 {
        for b in 0..=255u8 {
            table.set_transition(state, b, 0);
        }
    }
    let minimized = table.minimize().expect("should minimize");
    assert_eq!(minimized.state_count(), 1);
    assert_eq!(JitDfa::compile(&minimized).unwrap().scan_count(b"xxx"), 0);
}

#[test]
fn minimize_single_state_accept() {
    let mut table = new_table(1, 256);
    for b in 0..=255u8 {
        table.set_transition(0, b, 0);
    }
    table.add_accept(0, 0);
    table.set_pattern_length(0, 1);
    assert!(table.minimize().is_none());
    assert_eq!(JitDfa::compile(&table).unwrap().scan_count(b"abc"), 3);
}

#[test]
fn serialization_round_trip_1000_patterns() {
    let mut table = new_table(1001, 256);
    for pid in 0..1000 {
        let byte = u8::try_from(pid % 256).unwrap_or(0);
        let state = u32::try_from(pid + 1).unwrap_or(0);
        table.set_transition(0, byte, state);
        table.add_accept(state, u32::try_from(pid).unwrap_or(0));
        table.set_pattern_length(u32::try_from(pid).unwrap_or(0), 1);
    }

    let bytes = table.to_bytes();
    let restored = TransitionTable::from_bytes(&bytes).unwrap();
    assert_eq!(restored.state_count(), table.state_count());
    assert_eq!(restored.class_count(), table.class_count());
    assert_eq!(restored.transitions(), table.transitions());
    assert_eq!(restored.accept_states(), table.accept_states());
    assert_eq!(restored.pattern_lengths(), table.pattern_lengths());

    let jit_orig = JitDfa::compile(&table).unwrap();
    let jit_restored = JitDfa::compile(&restored).unwrap();
    let input = vec![0u8, 1u8, 2u8, 255u8];
    assert_eq!(jit_orig.scan_count(&input), jit_restored.scan_count(&input));
}

#[test]
fn jit_interpreted_parity_via_minimization() {
    let mut table = new_table(5000, 256);
    for state in 0..5000 {
        for b in 0..=255u8 {
            table.set_transition(state, b, 0);
        }
    }
    table.set_transition(0, b'x', 1);
    table.add_accept(1, 0);
    table.set_pattern_length(0, 1);

    let large_jit = JitDfa::compile(&table).unwrap();
    let minimized = table.minimize().expect("should minimize redundant states");
    assert!(minimized.state_count() <= 4096);

    let small_jit = JitDfa::compile(&minimized).unwrap();
    for input in [b"".as_slice(), b"x", b"xx", b"abc", b"xxxxxxxxxx"] {
        assert_eq!(large_jit.scan_count(input), small_jit.scan_count(input));
    }
}

#[test]
fn thread_safety_8_threads_many_patterns() {
    use std::sync::Arc;
    use std::thread;

    let patterns: Vec<Vec<u8>> = (0..100).map(|i| format!("p{i:02}").into_bytes()).collect();
    let pattern_refs: Vec<&[u8]> = patterns.iter().map(Vec::as_slice).collect();
    let jit = Arc::new(JitDfa::from_patterns(&pattern_refs).unwrap());

    let mut handles = vec![];
    for _ in 0..8 {
        let jit_clone = Arc::clone(&jit);
        handles.push(thread::spawn(move || {
            let input = b"p00 p01 p99 xyz";
            let mut matches = vec![Match::from_parts(0, 0, 0); 10];
            for _ in 0..100 {
                assert_eq!(jit_clone.scan(input, &mut matches), 3);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
