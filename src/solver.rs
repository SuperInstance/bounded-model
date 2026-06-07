//! DPLL SAT solver.
//!
//! A simple implementation of the DPLL algorithm with unit propagation and
//! pure literal elimination. Variables are represented as `i32` literals.
//!
//! The solver tracks statistics (decisions and propagations) and enforces a
//! maximum recursion depth equal to the number of variables.

use crate::cnf::CnfFormula;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of a SAT solver invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SatResult {
    /// Whether the formula is satisfiable.
    pub sat: bool,
    /// Variable assignment (index = variable number - 1).
    /// `None` if unsatisfiable.
    pub assignment: Option<Vec<bool>>,
    /// Number of branching decisions made.
    pub num_decisions: u64,
    /// Number of unit propagations performed.
    pub num_propagations: u64,
}

/// Solver statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SolverStats {
    pub num_decisions: u64,
    pub num_propagations: u64,
}

/// Solve a CNF formula using DPLL.
pub fn solve(formula: &CnfFormula) -> SatResult {
    let num_vars = formula.num_variables() as usize;
    if num_vars == 0 {
        // Empty formula or all empty clauses
        if formula.clauses().iter().any(|c| c.is_empty()) {
            return SatResult {
                sat: false,
                assignment: None,
                num_decisions: 0,
                num_propagations: 0,
            };
        }
        return SatResult {
            sat: true,
            assignment: Some(vec![]),
            num_decisions: 0,
            num_propagations: 0,
        };
    }

    let mut stats = SolverStats::default();
    let mut assignment: HashMap<i32, bool> = HashMap::new();

    let sat = dpll(formula.clauses(), &mut assignment, &mut stats, num_vars, 0);

    let assignment_vec = if sat {
        let mut v = vec![false; num_vars];
        for (var, val) in &assignment {
            if *var > 0 {
                v[(*var - 1) as usize] = *val;
            }
        }
        // Set default (false) for any unassigned vars
        Some(v)
    } else {
        None
    };

    SatResult {
        sat,
        assignment: assignment_vec,
        num_decisions: stats.num_decisions,
        num_propagations: stats.num_propagations,
    }
}

/// Check if a literal is satisfied under the current assignment.
fn lit_satisfied(lit: i32, assignment: &HashMap<i32, bool>) -> Option<bool> {
    let var = lit.abs();
    match assignment.get(&var) {
        Some(&val) => {
            if lit > 0 {
                Some(val)
            } else {
                Some(!val)
            }
        }
        None => None,
    }
}

/// Evaluate a clause: returns true if satisfied, false if falsified, None if undetermined.
fn _eval_clause(clause: &[i32], assignment: &HashMap<i32, bool>) -> Option<bool> {
    let mut has_unassigned = false;
    for &lit in clause {
        match lit_satisfied(lit, assignment) {
            Some(true) => return Some(true),
            Some(false) => {}
            None => has_unassigned = true,
        }
    }
    if has_unassigned {
        None
    } else {
        Some(false) // All literals falsified
    }
}

/// Simplify clauses by removing satisfied clauses and falsified literals.
fn simplify(clauses: &[Vec<i32>], assignment: &HashMap<i32, bool>) -> Vec<Vec<i32>> {
    clauses
        .iter()
        .filter_map(|clause| {
            // If any literal is satisfied, whole clause is satisfied → remove
            if clause
                .iter()
                .any(|&lit| lit_satisfied(lit, assignment) == Some(true))
            {
                return None;
            }
            // Remove falsified literals
            let simplified: Vec<i32> = clause
                .iter()
                .filter(|&&lit| lit_satisfied(lit, assignment) != Some(false))
                .copied()
                .collect();
            Some(simplified)
        })
        .collect()
}

/// Find all unit clauses (clauses with exactly one literal) and return their literals.
fn find_unit_clauses(clauses: &[Vec<i32>]) -> Vec<i32> {
    clauses
        .iter()
        .filter(|c| c.len() == 1)
        .map(|c| c[0])
        .collect()
}

/// Find pure literals: literals that appear with only one polarity.
fn find_pure_literals(clauses: &[Vec<i32>]) -> Vec<i32> {
    let mut pos = std::collections::HashSet::new();
    let mut neg = std::collections::HashSet::new();

    for clause in clauses {
        for &lit in clause {
            if lit > 0 {
                pos.insert(lit);
            } else {
                neg.insert(-lit);
            }
        }
    }

    let mut pure = Vec::new();
    for &v in &pos {
        if !neg.contains(&v) {
            pure.push(v); // appears only positive → assign true
        }
    }
    for &v in &neg {
        if !pos.contains(&v) {
            pure.push(-v); // appears only negative → assign false (literal -v means assign v=false)
        }
    }
    pure
}

/// Pick the next unassigned variable from the clauses.
fn pick_variable(clauses: &[Vec<i32>], assignment: &HashMap<i32, bool>) -> Option<i32> {
    for clause in clauses {
        for &lit in clause {
            let var = lit.abs();
            if !assignment.contains_key(&var) {
                return Some(var);
            }
        }
    }
    None
}

/// The core DPLL recursive solver.
fn dpll(
    clauses: &[Vec<i32>],
    assignment: &mut HashMap<i32, bool>,
    stats: &mut SolverStats,
    max_depth: usize,
    depth: usize,
) -> bool {
    // Enforce max recursion depth = number of variables
    if depth > max_depth {
        return false;
    }

    // Check for empty clause set → all satisfied
    if clauses.is_empty() {
        return true;
    }

    // Check for any empty clause (falsified)
    if clauses.iter().any(|c| c.is_empty()) {
        return false;
    }

    // Unit propagation
    let mut current_clauses = clauses.to_vec();
    loop {
        let units = find_unit_clauses(&current_clauses);
        if units.is_empty() {
            break;
        }
        for unit_lit in units {
            let var = unit_lit.abs();
            let val = unit_lit > 0;
            assignment.insert(var, val);
            stats.num_propagations += 1;
        }
        current_clauses = simplify(&current_clauses, assignment);

        // Check for conflict (empty clause)
        if current_clauses.iter().any(|c| c.is_empty()) {
            return false;
        }
        if current_clauses.is_empty() {
            return true;
        }
    }

    // Pure literal elimination
    let pure = find_pure_literals(&current_clauses);
    for lit in &pure {
        let var = lit.abs();
        if let std::collections::hash_map::Entry::Vacant(e) = assignment.entry(var) {
            let val = *lit > 0;
            e.insert(val);
            stats.num_propagations += 1;
        }
    }
    if !pure.is_empty() {
        current_clauses = simplify(&current_clauses, assignment);
        if current_clauses.iter().any(|c| c.is_empty()) {
            // Undo pure literal assignments
            for lit in &pure {
                assignment.remove(&lit.abs());
            }
            return false;
        }
        if current_clauses.is_empty() {
            return true;
        }
    }

    // Pick a variable to branch on
    match pick_variable(&current_clauses, assignment) {
        Some(var) => {
            stats.num_decisions += 1;

            // Try true first
            let mut assign_true = assignment.clone();
            assign_true.insert(var, true);
            let simplified = simplify(&current_clauses, &assign_true);
            if dpll(&simplified, &mut assign_true, stats, max_depth, depth + 1) {
                *assignment = assign_true;
                return true;
            }

            // Try false
            let mut assign_false = assignment.clone();
            assign_false.insert(var, false);
            let simplified = simplify(&current_clauses, &assign_false);
            if dpll(&simplified, &mut assign_false, stats, max_depth, depth + 1) {
                *assignment = assign_false;
                return true;
            }

            // Undo pure literal assignments on failure
            for lit in &pure {
                assignment.remove(&lit.abs());
            }

            false
        }
        None => {
            // All variables assigned, check if any clause is falsified
            !current_clauses.iter().any(|c| c.is_empty())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trivially_sat_empty() {
        let f = CnfFormula::new();
        let result = solve(&f);
        assert!(result.sat);
    }

    #[test]
    fn single_var_sat() {
        let f = CnfFormula::new().add_clause(vec![1]);
        let result = solve(&f);
        assert!(result.sat);
        assert_eq!(result.assignment.as_ref().unwrap()[0], true);
    }

    #[test]
    fn single_var_unsat() {
        let f = CnfFormula::new().add_clause(vec![1]).add_clause(vec![-1]);
        let result = solve(&f);
        assert!(!result.sat);
    }

    #[test]
    fn two_var_simple() {
        // (x1 ∨ x2) ∧ (¬x1 ∨ x2) ∧ (x1 ∨ ¬x2) ∧ (¬x1 ∨ ¬x2)
        // Only solution: x1=T, x2=F works? No. Let's use a simpler one.
        // (x1) ∧ (x2)
        let f = CnfFormula::new().add_clause(vec![1]).add_clause(vec![2]);
        let result = solve(&f);
        assert!(result.sat);
        let a = result.assignment.unwrap();
        assert!(a[0]);
        assert!(a[1]);
    }

    #[test]
    fn simple_unsat() {
        // (x1) ∧ (¬x1)
        let f = CnfFormula::new().add_clause(vec![1]).add_clause(vec![-1]);
        let result = solve(&f);
        assert!(!result.sat);
    }

    #[test]
    fn three_var_sat() {
        // (x1 ∨ x2 ∨ x3) ∧ (¬x1 ∨ ¬x2 ∨ x3)
        let f = CnfFormula::new()
            .add_clause(vec![1, 2, 3])
            .add_clause(vec![-1, -2, 3]);
        let result = solve(&f);
        assert!(result.sat);
    }

    #[test]
    fn tracks_decisions() {
        let f = CnfFormula::new()
            .add_clause(vec![1, 2])
            .add_clause(vec![1, -2])
            .add_clause(vec![-1, 2])
            .add_clause(vec![-1, -2]);
        let result = solve(&f);
        assert!(!result.sat);
        assert!(result.num_decisions > 0);
    }

    #[test]
    fn tracks_propagations() {
        // Unit clause forces propagation
        let f = CnfFormula::new().add_clause(vec![1]).add_clause(vec![1, 2]);
        let result = solve(&f);
        assert!(result.sat);
        assert!(result.num_propagations > 0);
    }

    #[test]
    fn empty_clause_unsat() {
        let f = CnfFormula::new().add_clause(vec![]);
        let result = solve(&f);
        assert!(!result.sat);
    }

    #[test]
    fn pure_literal() {
        // x1 appears only positive → pure literal elimination assigns x1=true
        let f = CnfFormula::new()
            .add_clause(vec![1, 2])
            .add_clause(vec![1, -2]);
        let result = solve(&f);
        assert!(result.sat);
    }

    #[test]
    fn tautology_clause() {
        // (x1 ∨ ¬x1) — always satisfied
        let f = CnfFormula::new().add_clause(vec![1, -1]);
        let result = solve(&f);
        assert!(result.sat);
    }

    #[test]
    fn complex_formula_sat() {
        // A more complex satisfiable formula
        let f = CnfFormula::new()
            .add_clause(vec![1, 2, 3])
            .add_clause(vec![-1, -2])
            .add_clause(vec![-1, -3])
            .add_clause(vec![-2, -3]);
        // x1=T,x2=F,x3=F satisfies
        let result = solve(&f);
        assert!(result.sat);
    }

    #[test]
    fn complex_formula_unsat() {
        // Pigeon hole: 3 pigeons, 2 holes (unsat)
        // Vars: p1h1=1, p1h2=2, p2h1=3, p2h2=4, p3h1=5, p3h2=6
        // Each pigeon in at least one hole
        let f = CnfFormula::new()
            .add_clause(vec![1, 2]) // p1 in h1 or h2
            .add_clause(vec![3, 4]) // p2 in h1 or h2
            .add_clause(vec![5, 6]) // p3 in h1 or h2
            // Each hole has at most one pigeon
            .add_clause(vec![-1, -3]) // not (p1h1 and p2h1)
            .add_clause(vec![-1, -5]) // not (p1h1 and p3h1)
            .add_clause(vec![-3, -5]) // not (p2h1 and p3h1)
            .add_clause(vec![-2, -4]) // not (p1h2 and p2h2)
            .add_clause(vec![-2, -6]) // not (p1h2 and p3h2)
            .add_clause(vec![-4, -6]); // not (p2h2 and p3h2)
        let result = solve(&f);
        assert!(!result.sat);
    }

    #[test]
    fn result_serialization() {
        let r = SatResult {
            sat: true,
            assignment: Some(vec![true, false]),
            num_decisions: 1,
            num_propagations: 3,
        };
        let json = serde_json::to_string(&r).unwrap();
        let r2: SatResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn stats_default() {
        let s = SolverStats::default();
        assert_eq!(s.num_decisions, 0);
        assert_eq!(s.num_propagations, 0);
    }

    #[test]
    fn many_unit_propagations() {
        // Chain: x1 → x2 → x3 → x4 → x5
        let mut f = CnfFormula::new().add_clause(vec![1]); // unit: x1=true
        for i in 1..5 {
            f = f.add_clause(vec![i, -(i + 1)]); // x_i → ¬x_i+1 means (¬x_i ∨ ¬x_{i+1})... 
        }
        let result = solve(&f);
        assert!(result.sat);
    }

    #[test]
    fn assignment_size_matches_vars() {
        let f = CnfFormula::new()
            .add_clause(vec![1, 2])
            .add_clause(vec![-1, 3])
            .add_clause(vec![-2, -3]);
        let result = solve(&f);
        assert!(result.sat);
        assert_eq!(result.assignment.as_ref().unwrap().len(), 3);
    }
}
