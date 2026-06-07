//! Encoding of state transition systems into CNF.
//!
//! This module provides functions to encode a simple state transition system
//! as a CNF formula suitable for bounded model checking. The encoding includes:
//!
//! - **Initial state constraints**: Variables at timestep 0 must represent an initial state.
//! - **Transition relation**: Variables at step *k* must follow from step *k-1*.
//! - **Negated property**: The property is negated at the final step (to find violations).
//!
//! ## Variable Encoding
//!
//! For `n` state variables and `k` timesteps:
//! - State variable `i` at timestep `t` is encoded as CNF variable `i + t * n + 1`.
//!
//! # Example
//!
//! ```
//! use bounded_model::encoding::{encode_bmc, TransitionEncoding};
//!
//! let enc = TransitionEncoding::new(2) // 2 boolean state vars
//!     .initial(vec![vec![1], vec![2]])           // x1=T, x2=T at step 0
//!     .transition(vec![
//!         // If x1=T at t, then x1=T at t+1 AND x2=F at t+1
//!         vec![-1, 3],   // current: var1, next: var3
//!         vec![-1, -4],  // current: var1, next: ¬var4
//!     ])
//!     .negated_property(vec![vec![-1]]); // property: x1 should be F, negate: x1=T
//!
//! let formula = enc.encode(3); // unroll 3 steps
//! ```

use crate::cnf::CnfFormula;
use serde::{Deserialize, Serialize};

/// Encoding of a transition system into CNF.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionEncoding {
    /// Number of boolean state variables.
    pub num_vars: usize,
    /// Clauses constraining the initial state (at timestep 0).
    pub initial: Vec<Vec<i32>>,
    /// Transition relation clauses (parameterized over current/next step).
    /// These are written in terms of variable indices 1..num_vars (current)
    /// and (num_vars+1)..(2*num_vars) (next).
    pub transition: Vec<Vec<i32>>,
    /// Negated property clauses at the final timestep.
    pub property: Vec<Vec<i32>>,
}

impl TransitionEncoding {
    /// Create a new encoding with the given number of state variables.
    pub fn new(num_vars: usize) -> Self {
        Self {
            num_vars,
            initial: Vec::new(),
            transition: Vec::new(),
            property: Vec::new(),
        }
    }

    /// Set the initial state constraints.
    pub fn initial(mut self, clauses: Vec<Vec<i32>>) -> Self {
        self.initial = clauses;
        self
    }

    /// Set the transition relation clauses.
    ///
    /// Variables 1..num_vars represent the current state,
    /// variables (num_vars+1)..(2*num_vars) represent the next state.
    pub fn transition(mut self, clauses: Vec<Vec<i32>>) -> Self {
        self.transition = clauses;
        self
    }

    /// Set the negated property clauses (checked at the final timestep).
    pub fn negated_property(mut self, clauses: Vec<Vec<i32>>) -> Self {
        self.property = clauses;
        self
    }

    /// Encode the BMC problem for `k` steps.
    ///
    /// Returns a CNF formula that is satisfiable iff there exists
    /// a path of length `k` from an initial state to a property violation.
    pub fn encode(&self, k: usize) -> CnfFormula {
        let mut formula = CnfFormula::new();

        // Initial state at timestep 0 (variables 1..num_vars)
        for clause in &self.initial {
            formula = formula.add_clause(clause.clone());
        }

        // Unroll transition relation for k steps
        for t in 0..k {
            for clause in &self.transition {
                let shifted: Vec<i32> = clause
                    .iter()
                    .map(|&lit| {
                        let var = lit.unsigned_abs() as usize;
                        let sign = if lit > 0 { 1i32 } else { -1i32 };
                        if var <= self.num_vars {
                            sign * (var + t * self.num_vars) as i32
                        } else {
                            sign * ((var - self.num_vars) + (t + 1) * self.num_vars) as i32
                        }
                    })
                    .collect();
                formula = formula.add_clause(shifted);
            }
        }

        // Negated property at final step
        let final_step = k;
        for clause in &self.property {
            let shifted: Vec<i32> = clause
                .iter()
                .map(|&lit| {
                    let var = lit.unsigned_abs() as usize;
                    let sign = if lit > 0 { 1i32 } else { -1i32 };
                    sign * (var + final_step * self.num_vars) as i32
                })
                .collect();
            formula = formula.add_clause(shifted);
        }

        formula
    }
}

/// Convenience function: encode and return the CNF formula.
pub fn encode_bmc(
    num_vars: usize,
    initial: Vec<Vec<i32>>,
    transition: Vec<Vec<i32>>,
    property: Vec<Vec<i32>>,
    k: usize,
) -> CnfFormula {
    TransitionEncoding::new(num_vars)
        .initial(initial)
        .transition(transition)
        .negated_property(property)
        .encode(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_encoding() {
        // 2 vars, 1 step
        let enc = TransitionEncoding::new(2)
            .initial(vec![vec![1], vec![2]])
            .transition(vec![vec![-1, 3], vec![-2, 4]])
            .negated_property(vec![vec![-3]]);

        let f = enc.encode(1);
        // Initial: [1], [2]
        // Transition at t=0: [-1, 3] (shifted: [-1, 3]), [-2, 4] (shifted: [-2, 4])
        // Property at step 1: [-3] (shifted: var 1 + 1*2 = 3) → [-3]
        assert!(f.num_clauses() > 0);
    }

    #[test]
    fn encoding_zero_steps() {
        let enc = TransitionEncoding::new(1)
            .initial(vec![vec![1]])
            .transition(vec![])
            .negated_property(vec![vec![-1]]);
        let f = enc.encode(0);
        // Only initial + property at step 0
        assert_eq!(f.num_clauses(), 2); // [1] and [-1]
    }

    #[test]
    fn encoding_two_steps() {
        let enc = TransitionEncoding::new(2)
            .initial(vec![vec![1]])
            .transition(vec![vec![-1, 3]])
            .negated_property(vec![vec![-3]]);

        let f = enc.encode(2);
        // Initial: 1 clause
        // Transition t=0: [-1, 3] (v1=1, v3=1+2=3) → [-1, 3]
        // Transition t=1: [-1, 3] → [-(1+1*2), 3+2*2] = [-3, 7]... wait
        // Actually: at t=1, current vars are at offset 1*2=2, next at offset 2*2=4
        // clause [-1, 3]: var 1 → 1+1*2=3, var 3 → (3-2)+2*2=5 → [-3, 5]
        assert!(f.num_clauses() >= 3);
    }

    #[test]
    fn encode_bmc_function() {
        let f = encode_bmc(1, vec![vec![1]], vec![vec![-1, 2]], vec![vec![-1]], 1);
        assert!(f.num_clauses() > 0);
    }

    #[test]
    fn encoding_preserves_satisfiability() {
        // Simple: x1 starts true, stays true, property violation if x1 is false
        // With negated property = [1] (meaning we assert x1=true at final step)
        // This should be satisfiable trivially
        let enc = TransitionEncoding::new(1)
            .initial(vec![vec![1]]) // x1 = true at step 0
            .transition(vec![vec![-1, 2]]) // if x1 at t, then x1 at t+1
            .negated_property(vec![vec![1]]); // assert x1=true at final step

        let f = enc.encode(1);
        let result = crate::solver::solve(&f);
        assert!(result.sat);
    }

    #[test]
    fn transition_encoding_serialization() {
        let enc = TransitionEncoding::new(2)
            .initial(vec![vec![1]])
            .transition(vec![vec![-1, 3]])
            .negated_property(vec![vec![-1]]);

        let json = serde_json::to_string(&enc).unwrap();
        let enc2: TransitionEncoding = serde_json::from_str(&json).unwrap();
        assert_eq!(enc2.num_vars, 2);
    }
}
