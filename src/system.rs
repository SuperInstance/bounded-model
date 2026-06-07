//! Transition system definition.
//!
//! A simple finite-state transition system with named states,
//! explicit transitions, and a property to check.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A finite-state transition system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSystem {
    /// Named states of the system.
    pub states: Vec<String>,
    /// Indices of initial states.
    pub initial: Vec<usize>,
    /// Transition relation: from_state → Vec<to_state>.
    pub transitions: HashMap<usize, Vec<usize>>,
    /// Property to check: returns true if the state VIOLATES the specification.
    /// Input is the state index.
    pub property: Option<PropertyFn>,
}

/// A serializable property function representation.
///
/// Since we can't serialize closures, we store a string identifier
/// that maps to a known property check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyFn {
    /// Check that the state index never equals the given value.
    NeverReach(usize),
    /// Check that the state name never contains the given substring.
    NeverContain(String),
    /// Always eventually reaches the given state.
    AlwaysReach(usize),
    /// Custom property identified by name with parameters.
    Custom { name: String, params: Vec<String> },
}

impl PropertyFn {
    /// Evaluate the property. Returns `true` if the state **violates** the spec.
    pub fn check(&self, state_idx: usize, state_name: &str) -> bool {
        match self {
            PropertyFn::NeverReach(idx) => state_idx == *idx,
            PropertyFn::NeverContain(s) => state_name.contains(s),
            PropertyFn::AlwaysReach(_) => false, // Handled differently in BMC
            PropertyFn::Custom { .. } => false,  // Placeholder
        }
    }
}

/// Builder for constructing transition systems.
#[derive(Debug, Clone, Default)]
pub struct TransitionSystemBuilder {
    states: Vec<String>,
    initial: Vec<usize>,
    transitions: HashMap<usize, Vec<usize>>,
    property: Option<PropertyFn>,
}

impl TransitionSystemBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a named state. Returns the state's index.
    pub fn add_state(&mut self, name: &str) -> usize {
        let idx = self.states.len();
        self.states.push(name.to_string());
        idx
    }

    /// Mark a state as initial.
    pub fn add_initial(&mut self, state_idx: usize) {
        if !self.initial.contains(&state_idx) {
            self.initial.push(state_idx);
        }
    }

    /// Add a transition from one state to another.
    pub fn add_transition(&mut self, from: usize, to: usize) {
        self.transitions.entry(from).or_default().push(to);
    }

    /// Set the property function.
    pub fn property(&mut self, prop: PropertyFn) {
        self.property = Some(prop);
    }

    /// Build the transition system.
    pub fn build(self) -> TransitionSystem {
        TransitionSystem {
            states: self.states,
            initial: self.initial,
            transitions: self.transitions,
            property: self.property,
        }
    }
}

impl TransitionSystem {
    /// Create using a builder.
    pub fn builder() -> TransitionSystemBuilder {
        TransitionSystemBuilder::new()
    }

    /// Get the name of a state by index.
    pub fn state_name(&self, idx: usize) -> Option<&str> {
        self.states.get(idx).map(|s| s.as_str())
    }

    /// Check if a state is initial.
    pub fn is_initial(&self, idx: usize) -> bool {
        self.initial.contains(&idx)
    }

    /// Get the successors of a state.
    pub fn successors(&self, idx: usize) -> &[usize] {
        self.transitions
            .get(&idx)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Count total states.
    pub fn num_states(&self) -> usize {
        self.states.len()
    }

    /// Count total transitions.
    pub fn num_transitions(&self) -> usize {
        self.transitions.values().map(|v| v.len()).sum()
    }

    /// Check if a state violates the property.
    pub fn violates_property(&self, state_idx: usize) -> bool {
        match &self.property {
            Some(prop) => {
                let name = self.state_name(state_idx).unwrap_or("?");
                prop.check(state_idx, name)
            }
            None => false,
        }
    }

    /// Find all reachable states from the initial states.
    pub fn reachable_states(&self) -> Vec<usize> {
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<usize> = self.initial.clone();

        while let Some(s) = stack.pop() {
            if visited.insert(s)
                && let Some(succs) = self.transitions.get(&s)
            {
                for &succ in succs {
                    if !visited.contains(&succ) {
                        stack.push(succ);
                    }
                }
            }
        }

        let mut result: Vec<usize> = visited.into_iter().collect();
        result.sort();
        result
    }

    /// Encode as a simple boolean transition system for BMC.
    ///
    /// Each state is encoded as a one-hot boolean vector.
    /// Returns a simplified encoding suitable for small systems.
    pub fn to_encoding(&self) -> crate::encoding::TransitionEncoding {
        let n = self.states.len();
        let num_vars = n; // One variable per state (one-hot)

        // Initial state: at least one initial state is active
        let mut initial_clauses = vec![self.initial.iter().map(|&i| (i + 1) as i32).collect()];

        // Exactly-one constraints for initial states
        for i in 0..self.initial.len() {
            for j in (i + 1)..self.initial.len() {
                let a = self.initial[i];
                let b = self.initial[j];
                initial_clauses.push(vec![-((a + 1) as i32), -((b + 1) as i32)]);
            }
        }

        // Transition relation
        let mut trans_clauses = Vec::new();
        for (&from, tos) in &self.transitions {
            let from_var = (from + 1) as i32;

            // If state `from` is active, at least one successor must be active
            let succ_next: Vec<i32> = tos.iter().map(|&t| (t + 1 + n) as i32).collect();
            if !succ_next.is_empty() {
                let mut clause = vec![-from_var];
                clause.extend(succ_next);
                trans_clauses.push(clause);
            }

            // Mutual exclusion: at most one successor
            for i in 0..tos.len() {
                for j in (i + 1)..tos.len() {
                    trans_clauses.push(vec![
                        -from_var,
                        -((tos[i] + 1 + n) as i32),
                        -((tos[j] + 1 + n) as i32),
                    ]);
                }
            }
        }

        // Negated property: the property is violated at the final step
        let property_clauses: Vec<Vec<i32>> = self
            .states
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.violates_property(*idx))
            .map(|(idx, _)| vec![(idx + 1) as i32])
            .collect();

        crate::encoding::TransitionEncoding::new(num_vars)
            .initial(initial_clauses)
            .transition(trans_clauses)
            .negated_property(property_clauses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_simple_system() -> TransitionSystem {
        let mut b = TransitionSystem::builder();
        let s0 = b.add_state("idle");
        let s1 = b.add_state("running");
        let s2 = b.add_state("error");
        b.add_initial(s0);
        b.add_transition(s0, s1);
        b.add_transition(s1, s1);
        b.add_transition(s1, s2);
        b.add_transition(s2, s2);
        b.property(PropertyFn::NeverReach(s2));
        b.build()
    }

    #[test]
    fn builder_creates_system() {
        let sys = build_simple_system();
        assert_eq!(sys.num_states(), 3);
        assert_eq!(sys.num_transitions(), 4);
    }

    #[test]
    fn initial_state() {
        let sys = build_simple_system();
        assert!(sys.is_initial(0));
        assert!(!sys.is_initial(1));
    }

    #[test]
    fn state_names() {
        let sys = build_simple_system();
        assert_eq!(sys.state_name(0), Some("idle"));
        assert_eq!(sys.state_name(1), Some("running"));
        assert_eq!(sys.state_name(2), Some("error"));
    }

    #[test]
    fn successors() {
        let sys = build_simple_system();
        assert_eq!(sys.successors(0), &[1]);
        assert_eq!(sys.successors(1), &[1, 2]);
        assert_eq!(sys.successors(2), &[2]);
    }

    #[test]
    fn property_violation() {
        let sys = build_simple_system();
        assert!(!sys.violates_property(0)); // idle
        assert!(!sys.violates_property(1)); // running
        assert!(sys.violates_property(2)); // error (violates NeverReach(2))
    }

    #[test]
    fn reachable_states() {
        let sys = build_simple_system();
        let reachable = sys.reachable_states();
        assert_eq!(reachable, vec![0, 1, 2]);
    }

    #[test]
    fn to_encoding() {
        let sys = build_simple_system();
        let enc = sys.to_encoding();
        assert!(enc.num_vars > 0);
        assert!(!enc.initial.is_empty());
        assert!(!enc.transition.is_empty());
    }

    #[test]
    fn property_never_contain() {
        let prop = PropertyFn::NeverContain("err".to_string());
        assert!(prop.check(2, "error"));
        assert!(!prop.check(0, "idle"));
    }

    #[test]
    fn system_serialization() {
        let sys = build_simple_system();
        let json = serde_json::to_string(&sys).unwrap();
        let sys2: TransitionSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(sys2.num_states(), 3);
        assert_eq!(sys2.states, sys.states);
    }

    #[test]
    fn custom_property() {
        let prop = PropertyFn::Custom {
            name: "my_prop".to_string(),
            params: vec!["1".to_string()],
        };
        assert!(!prop.check(0, "anything")); // placeholder returns false
    }

    #[test]
    fn no_property_no_violation() {
        let sys = TransitionSystem {
            states: vec!["a".to_string()],
            initial: vec![0],
            transitions: HashMap::new(),
            property: None,
        };
        assert!(!sys.violates_property(0));
    }

    #[test]
    fn always_reach_property() {
        let prop = PropertyFn::AlwaysReach(1);
        // AlwaysReach is handled differently in BMC, check returns false
        assert!(!prop.check(0, "idle"));
    }
}
