//! CNF formula representation.
//!
//! A CNF (Conjunctive Normal Form) formula is a conjunction of clauses,
//! where each clause is a disjunction of literals. A literal is a signed
//! variable: positive means the variable is true, negative means false.
//!
//! # Example
//!
//! ```
//! use bounded_model::cnf::CnfFormula;
//!
//! let formula = CnfFormula::new()
//!     .add_clause(vec![1, -2])      // (x1 ∨ ¬x2)
//!     .add_clause(vec![-1, 2, 3]);  // (¬x1 ∨ x2 ∨ x3)
//!
//! assert_eq!(formula.num_clauses(), 2);
//! assert_eq!(formula.num_variables(), 3);
//! ```

use serde::{Deserialize, Serialize};

/// A CNF formula represented as a list of clauses.
///
/// Each clause is a `Vec<i32>` of literals. Literal `+v` means variable `v`
/// is true; `-v` means variable `v` is false.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CnfFormula {
    /// The clauses of the formula.
    clauses: Vec<Vec<i32>>,
}

impl CnfFormula {
    /// Create a new empty CNF formula.
    pub fn new() -> Self {
        Self {
            clauses: Vec::new(),
        }
    }

    /// Add a clause (disjunction of literals).
    pub fn add_clause(mut self, clause: Vec<i32>) -> Self {
        self.clauses.push(clause);
        self
    }

    /// Add multiple clauses at once.
    pub fn add_clauses(mut self, clauses: Vec<Vec<i32>>) -> Self {
        self.clauses.extend(clauses);
        self
    }

    /// Get a reference to the clauses.
    pub fn clauses(&self) -> &[Vec<i32>] {
        &self.clauses
    }

    /// Number of clauses.
    pub fn num_clauses(&self) -> usize {
        self.clauses.len()
    }

    /// Number of distinct variables (determined by the maximum absolute literal value).
    pub fn num_variables(&self) -> u32 {
        self.clauses
            .iter()
            .flat_map(|c| c.iter().map(|l| l.unsigned_abs()))
            .max()
            .unwrap_or(0)
    }

    /// Check if the formula is empty (no clauses).
    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }
}

impl Default for CnfFormula {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_formula() {
        let f = CnfFormula::new();
        assert!(f.is_empty());
        assert_eq!(f.num_clauses(), 0);
        assert_eq!(f.num_variables(), 0);
    }

    #[test]
    fn single_clause() {
        let f = CnfFormula::new().add_clause(vec![1, -2, 3]);
        assert_eq!(f.num_clauses(), 1);
        assert_eq!(f.num_variables(), 3);
    }

    #[test]
    fn multiple_clauses() {
        let f = CnfFormula::new()
            .add_clause(vec![1])
            .add_clause(vec![-1, 2])
            .add_clause(vec![-2, 3]);
        assert_eq!(f.num_clauses(), 3);
        assert_eq!(f.num_variables(), 3);
    }

    #[test]
    fn builder_chain() {
        let f = CnfFormula::new()
            .add_clause(vec![1, 2])
            .add_clause(vec![-1, -2]);
        assert_eq!(f.clauses().len(), 2);
    }

    #[test]
    fn add_clauses_batch() {
        let f = CnfFormula::new().add_clauses(vec![vec![1, 2], vec![-1, -2]]);
        assert_eq!(f.num_clauses(), 2);
    }

    #[test]
    fn default_is_new() {
        let f1 = CnfFormula::new();
        let f2 = CnfFormula::default();
        assert_eq!(f1, f2);
    }

    #[test]
    fn num_variables_with_gaps() {
        // Variables 1, 3, 7 used → max is 7
        let f = CnfFormula::new()
            .add_clause(vec![1, 3])
            .add_clause(vec![-7]);
        assert_eq!(f.num_variables(), 7);
    }

    #[test]
    fn serialize_deserialize() {
        let f = CnfFormula::new()
            .add_clause(vec![1, -2])
            .add_clause(vec![-1, 2, 3]);
        let json = serde_json::to_string(&f).unwrap();
        let f2: CnfFormula = serde_json::from_str(&json).unwrap();
        assert_eq!(f, f2);
    }
}
