use super::TransitionTable;

impl TransitionTable {
    /// Minimize the DFA using partition refinement.
    #[must_use]
    pub fn minimize(&self) -> Option<Self> {
        if self.state_count <= 1 {
            return None;
        }

        let mut state_to_pattern: std::collections::HashMap<u32, u32> =
            std::collections::HashMap::new();
        for &(s, pid) in &self.accept_states {
            state_to_pattern.insert(s, pid);
        }

        let mut partition = vec![0u32; self.state_count];
        let mut next_class = 1u32;
        let mut pattern_class: std::collections::HashMap<u32, u32> =
            std::collections::HashMap::new();
        for i in 0..self.state_count {
            if let Some(&pid) = state_to_pattern.get(&(i as u32)) {
                let class = *pattern_class.entry(pid).or_insert_with(|| {
                    let c = next_class;
                    next_class += 1;
                    c
                });
                partition[i] = class;
            }
        }
        let mut num_classes = next_class;

        let mut changed = true;
        while changed {
            changed = false;
            let mut new_partition = partition.clone();
            let mut signature_map: std::collections::HashMap<Vec<u32>, u32> =
                std::collections::HashMap::new();
            let mut next_class = 0u32;

            for state in 0..self.state_count {
                let current_class = partition[state];
                let mut sig = Vec::with_capacity(self.class_count + 1);
                sig.push(current_class);
                for byte in 0..self.class_count {
                    let idx = state * self.class_count + byte;
                    let target = self.transitions[idx] as usize;
                    let target_class = if target < self.state_count {
                        partition[target]
                    } else {
                        0
                    };
                    sig.push(target_class);
                }

                let class = if let Some(&existing) = signature_map.get(&sig) {
                    existing
                } else {
                    let c = next_class;
                    signature_map.insert(sig, c);
                    next_class += 1;
                    c
                };
                new_partition[state] = class;
            }

            if next_class != num_classes || new_partition != partition {
                changed = true;
                num_classes = next_class;
                partition = new_partition;
            }
        }

        let new_state_count = num_classes as usize;
        if new_state_count >= self.state_count {
            return None;
        }

        let mut new_table = Self::new(new_state_count, self.class_count).ok()?;
        let mut class_representative = vec![0usize; new_state_count];
        for (state, &class) in partition.iter().enumerate() {
            class_representative[class as usize] = state;
        }

        for new_state in 0..new_state_count {
            let repr = class_representative[new_state];
            for byte in 0..self.class_count {
                let idx = repr * self.class_count + byte;
                let old_target = self.transitions[idx] as usize;
                let new_target = if old_target < self.state_count {
                    partition[old_target]
                } else {
                    0
                };
                new_table.transitions[new_state * self.class_count + byte] = new_target;
            }
        }

        for &(old_state, pattern_id) in &self.accept_states {
            let new_state = partition[old_state as usize];
            if !new_table
                .accept_states
                .iter()
                .any(|&(s, p)| s == new_state && p == pattern_id)
            {
                new_table.add_accept(new_state, pattern_id);
            }
        }

        new_table.pattern_lengths.clone_from(&self.pattern_lengths);
        Some(new_table)
    }
}
