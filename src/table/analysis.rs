use super::TransitionTable;

impl TransitionTable {
    /// Collapse each state's byte transitions into maximal consecutive ranges.
    #[must_use]
    pub fn compute_ranges(&self) -> Vec<Vec<(u8, u8, u32)>> {
        let mut ranges = Vec::with_capacity(self.state_count);
        if self.class_count == 0 {
            return ranges;
        }

        for state in 0..self.state_count {
            let row_start = state.saturating_mul(self.class_count);
            let row_end = row_start
                .saturating_add(self.class_count)
                .min(self.transitions.len());
            let row = &self.transitions[row_start..row_end];
            let limit = row.len().min(usize::from(u8::MAX) + 1);
            if limit == 0 {
                ranges.push(Vec::new());
                continue;
            }

            let mut state_ranges = Vec::new();
            let mut start = 0usize;
            let mut target = row[0];
            for index in 1..limit {
                if row[index] != target {
                    state_ranges.push((start as u8, (index - 1) as u8, target));
                    start = index;
                    target = row[index];
                }
            }
            state_ranges.push((start as u8, (limit - 1) as u8, target));
            ranges.push(state_ranges);
        }

        ranges
    }

    /// Estimated JIT code size in bytes.
    #[must_use]
    pub fn estimated_code_size(&self) -> usize {
        let code = 256;
        let data =
            self.transitions.len() * 4 + self.state_count * 4 + self.pattern_lengths.len() * 4;
        code + data
    }

    /// Count distinct transition targets for a state.
    #[must_use]
    pub fn transition_density(&self, state: usize) -> usize {
        if state >= self.state_count {
            return 0;
        }
        let base = state * self.class_count;
        let mut targets = std::collections::HashSet::new();
        for byte in 0..self.class_count {
            if let Some(&t) = self.transitions.get(base + byte) {
                targets.insert(t);
            }
        }
        targets.len()
    }

    /// Whether this DFA is small enough for JIT compilation.
    #[must_use]
    pub fn is_jit_eligible(&self) -> bool {
        self.state_count <= 4096 && self.class_count == 256
    }
}
