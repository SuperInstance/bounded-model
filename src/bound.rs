//! Bounded model checking loop.
//!
//! Iteratively unrolls the transition relation for increasing bounds
//! k = 1, 2, 3, ... until either:
//!
//! - A counterexample is found (SAT result), or
//! - The maximum bound is reached.

use crate::cnf::CnfFormula;
use crate::encoding::TransitionEncoding;
use crate::solver;
use serde::{Deserialize, Serialize};

/// Result of bounded model checking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BmcResult {
    /// Whether a counterexample was found (SAT means property violation).
    pub sat: bool,
    /// The bound at which the result was determined.
    pub k: usize,
    /// The satisfying assignment (counterexample), if any.
    pub assignment: Option<Vec<bool>>,
}

/// Configuration for the bounded checker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BmcConfig {
    /// Maximum bound to check.
    pub max_k: usize,
    /// Whether to stop at the first counterexample.
    pub stop_on_sat: bool,
}

impl Default for BmcConfig {
    fn default() -> Self {
        Self {
            max_k: 10,
            stop_on_sat: true,
        }
    }
}

impl BmcConfig {
    /// Create a new config with the given maximum bound.
    pub fn new(max_k: usize) -> Self {
        Self {
            max_k,
            stop_on_sat: true,
        }
    }
}

/// Run bounded model checking on the given encoding.
pub fn check(encoding: &TransitionEncoding, config: &BmcConfig) -> BmcResult {
    for k in 0..=config.max_k {
        let formula = encoding.encode(k);
        let result = solver::solve(&formula);

        if result.sat {
            return BmcResult {
                sat: true,
                k,
                assignment: result.assignment,
            };
        }
    }

    BmcResult {
        sat: false,
        k: config.max_k,
        assignment: None,
    }
}

/// Run bounded model checking with a simple formula for each step.
///
/// This is a lower-level API where you provide a closure that generates
/// the CNF formula for each bound k.
pub fn check_with<F>(max_k: usize, formula_gen: F) -> BmcResult
where
    F: Fn(usize) -> CnfFormula,
{
    for k in 0..=max_k {
        let formula = formula_gen(k);
        let result = solver::solve(&formula);

        if result.sat {
            return BmcResult {
                sat: true,
                k,
                assignment: result.assignment,
            };
        }
    }

    BmcResult {
        sat: false,
        k: max_k,
        assignment: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immediate_counterexample() {
        // Property violated at k=0
        let enc = TransitionEncoding::new(1)
            .initial(vec![vec![1]])
            .transition(vec![])
            .negated_property(vec![vec![1]]); // property violation: x1=true

        let result = check(&enc, &BmcConfig::new(5));
        assert!(result.sat);
        assert_eq!(result.k, 0);
    }

    #[test]
    fn no_counterexample_within_bound() {
        // Property can never be violated (contradiction in negated property)
        let enc = TransitionEncoding::new(1)
            .initial(vec![vec![1]])
            .transition(vec![vec![-1, 2]])
            .negated_property(vec![vec![1], vec![-1]]); // x1 ∧ ¬x1 → always UNSAT

        let result = check(&enc, &BmcConfig::new(3));
        assert!(!result.sat);
    }

    #[test]
    fn counterexample_at_k1() {
        // x1 starts true, transition flips to false, property is x1 must be true
        let enc = TransitionEncoding::new(1)
            .initial(vec![vec![1]]) // x1 = true
            .transition(vec![vec![-1, -2]]) // if x1 then ¬x1' (flip)
            .negated_property(vec![vec![-1]]); // violation: ¬x1 at final step

        let result = check(&enc, &BmcConfig::new(5));
        assert!(result.sat);
    }

    #[test]
    fn bmc_result_serialization() {
        let r = BmcResult {
            sat: true,
            k: 3,
            assignment: Some(vec![true, false, true]),
        };
        let json = serde_json::to_string(&r).unwrap();
        let r2: BmcResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn check_with_closure() {
        let result = check_with(3, |_k| crate::cnf::CnfFormula::new().add_clause(vec![1, 2]));
        assert!(result.sat);
    }

    #[test]
    fn default_config() {
        let config = BmcConfig::default();
        assert_eq!(config.max_k, 10);
        assert!(config.stop_on_sat);
    }

    #[test]
    fn bmc_config_serialization() {
        let config = BmcConfig::new(20);
        let json = serde_json::to_string(&config).unwrap();
        let config2: BmcConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config2.max_k, 20);
    }
}
