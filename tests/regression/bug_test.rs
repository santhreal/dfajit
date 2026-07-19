use dfajit::{Error, JitDfa, TransitionTable};
use matchkit::Match;

#[test]
fn test_missing_failure_transitions() {
    let jit = JitDfa::from_patterns(&[b"foo"]).unwrap_or_else(|_| panic!("failed to build DFA"));
    let mut matches = vec![Match::from_parts(0, 0, 0); 10];
    let count = jit.scan(b"ffoo", &mut matches);
    assert_eq!(count, 1);
}

#[test]
fn compute_ranges_does_not_panic_on_short_transition_buffer() {
    // Regression (analysis.rs:17): a table whose transitions vector is shorter
    // than state_count * class_count (e.g. truncated after construction) made
    // compute_ranges slice transitions[row_start..row_end] with row_start >
    // row_end for out-of-range states, panicking. It must now yield empty rows
    // for those states instead.
    let mut table = TransitionTable::new(4, 256).expect("valid table");
    // Truncate the backing buffer so states 1..4 are out of range.
    table.transitions_raw_mut().truncate(10);
    let ranges = table.compute_ranges();
    assert_eq!(ranges.len(), 4, "one range vector per declared state");
    // The truncated (out-of-range) states collapse to empty range lists.
    assert!(
        ranges[1..].iter().all(std::vec::Vec::is_empty),
        "out-of-range states must produce empty ranges, not panic"
    );
}

#[test]
fn from_patterns_fails_fast_past_max_states() {
    // Regression (dfa.rs:342): a pattern longer than MAX_STATES used to allocate
    // the entire oversized trie (1 KiB per state) before build_dense_table
    // rejected it. It must fail with TooManyStates during construction.
    let oversized = vec![b'a'; TransitionTable::MAX_STATES + 8];
    let result = JitDfa::from_patterns(&[oversized.as_slice()]);
    match result {
        Err(Error::TooManyStates { states, max }) => {
            assert_eq!(max, TransitionTable::MAX_STATES);
            assert!(
                states > TransitionTable::MAX_STATES,
                "reported state count should exceed the cap"
            );
        }
        other => panic!("expected TooManyStates, got {other:?}"),
    }
}
