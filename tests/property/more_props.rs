use dfajit::TransitionTable;
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_serialization_round_trip(
        state_count in 1..100usize,
        class_count in 1..256usize,
    ) {
        let table = TransitionTable::new(state_count, class_count).unwrap();

        let bytes = table.to_bytes();
        let restored = TransitionTable::from_bytes(&bytes).unwrap();

        assert_eq!(table.state_count(), restored.state_count());
        assert_eq!(table.class_count(), restored.class_count());
        assert_eq!(table.transitions(), restored.transitions());
        assert_eq!(table.accept_states(), restored.accept_states());
        assert_eq!(table.pattern_lengths(), restored.pattern_lengths());
    }
}
