use dfajit::JitDfa;
use dfajit::TransitionTable;
use matchkit::Match;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_dfa_scan_never_panics(input in any::<Vec<u8>>()) {
        let mut table = TransitionTable::new(2, 256).unwrap();
        table.set_transition(0, b'x', 1);
        table.add_accept(1, 0);
        table.set_pattern_length(0, 1);

        let dfa = JitDfa::compile(&table).unwrap();
        let mut matches = vec![Match::from_parts(0, 0, 0); 100];

        // This should not panic for any input
        let count = dfa.scan(&input, &mut matches);

        // Check bounds on matches
        assert!(count <= input.len());
        for m in matches.iter().take(count) {
            assert!((m.start as usize) < input.len());
            assert!((m.end as usize) <= input.len());
            assert!(m.start < m.end);
            assert_eq!(m.pattern_id, 0);
        }
    }
}
