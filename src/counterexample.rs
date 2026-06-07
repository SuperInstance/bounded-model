//! Counterexample extraction.
//!
//! Given a satisfying assignment from the SAT solver, extract an execution
//! trace by mapping variable assignments back to states at each timestep.

use serde::{Deserialize, Serialize};

/// A single state in an execution trace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    /// Timestep of this state.
    pub timestep: usize,
    /// Variable assignments as (variable_index, value) pairs.
    pub variables: Vec<(usize, bool)>,
    /// Human-readable description.
    pub description: String,
}

/// An execution trace (sequence of states).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Trace {
    /// The states in the trace, ordered by timestep.
    pub states: Vec<State>,
}

/// Extract an execution trace from a SAT assignment.
///
/// # Arguments
///
/// * `assignment` - The SAT solver's variable assignment.
/// * `num_state_vars` - Number of boolean state variables in the system.
/// * `var_names` - Optional names for each state variable.
///
/// # Returns
///
/// A trace with one state per timestep.
pub fn extract_trace(assignment: &[bool], num_state_vars: usize, var_names: &[String]) -> Trace {
    if num_state_vars == 0 || assignment.is_empty() {
        return Trace { states: vec![] };
    }

    let num_steps = assignment.len() / num_state_vars;
    let mut states = Vec::with_capacity(num_steps);

    for step in 0..num_steps {
        let mut variables = Vec::with_capacity(num_state_vars);
        let mut descriptions = Vec::new();

        for var in 0..num_state_vars {
            let idx = step * num_state_vars + var;
            let val = if idx < assignment.len() {
                assignment[idx]
            } else {
                false
            };
            variables.push((var + 1, val));

            let name = var_names
                .get(var)
                .map(|s| s.as_str())
                .unwrap_or_else(|| "?");
            descriptions.push(format!("{}={}", name, if val { "T" } else { "F" }));
        }

        states.push(State {
            timestep: step,
            variables,
            description: descriptions.join(", "),
        });
    }

    Trace { states }
}

/// Format a trace as a human-readable string table.
pub fn format_trace(trace: &Trace) -> String {
    if trace.states.is_empty() {
        return "Empty trace".to_string();
    }

    let mut lines = Vec::new();
    lines.push(format!("Execution trace ({} steps):", trace.states.len()));
    lines.push("-".repeat(50));

    for state in &trace.states {
        lines.push(format!("Step {}: {}", state.timestep, state.description));
    }

    lines.join("\n")
}

/// Convert a trace to a simple Vec<Vec<String>> representation.
pub fn trace_to_table(trace: &Trace) -> Vec<Vec<String>> {
    trace
        .states
        .iter()
        .map(|s| {
            let mut row = vec![format!("Step {}", s.timestep)];
            row.extend(
                s.variables
                    .iter()
                    .map(|(v, val)| format!("var{}={}", v, if *val { "T" } else { "F" })),
            );
            row
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_var_names(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn empty_assignment() {
        let trace = extract_trace(&[], 2, &make_var_names(&["x", "y"]));
        assert!(trace.states.is_empty());
    }

    #[test]
    fn single_step() {
        let assignment = vec![true, false];
        let trace = extract_trace(&assignment, 2, &make_var_names(&["x", "y"]));
        assert_eq!(trace.states.len(), 1);
        assert_eq!(trace.states[0].timestep, 0);
        assert_eq!(trace.states[0].description, "x=T, y=F");
    }

    #[test]
    fn two_steps() {
        let assignment = vec![true, false, false, true];
        let trace = extract_trace(&assignment, 2, &make_var_names(&["x", "y"]));
        assert_eq!(trace.states.len(), 2);
        assert_eq!(trace.states[0].description, "x=T, y=F");
        assert_eq!(trace.states[1].description, "x=F, y=T");
    }

    #[test]
    fn default_var_names() {
        let assignment = vec![true];
        let trace = extract_trace(&assignment, 1, &[]);
        assert_eq!(trace.states[0].description, "?=T");
    }

    #[test]
    fn format_empty_trace() {
        let trace = Trace { states: vec![] };
        assert_eq!(format_trace(&trace), "Empty trace");
    }

    #[test]
    fn format_single_step() {
        let trace = Trace {
            states: vec![State {
                timestep: 0,
                variables: vec![(1, true)],
                description: "x=T".to_string(),
            }],
        };
        let output = format_trace(&trace);
        assert!(output.contains("Step 0"));
        assert!(output.contains("x=T"));
    }

    #[test]
    fn trace_to_table_format() {
        let trace = Trace {
            states: vec![
                State {
                    timestep: 0,
                    variables: vec![(1, true), (2, false)],
                    description: "x=T, y=F".to_string(),
                },
                State {
                    timestep: 1,
                    variables: vec![(1, false), (2, true)],
                    description: "x=F, y=T".to_string(),
                },
            ],
        };
        let table = trace_to_table(&trace);
        assert_eq!(table.len(), 2);
        assert!(table[0].contains(&"Step 0".to_string()));
        assert!(table[1].contains(&"Step 1".to_string()));
    }

    #[test]
    fn state_serialization() {
        let s = State {
            timestep: 1,
            variables: vec![(1, true), (2, false)],
            description: "x=T, y=F".to_string(),
        };
        let json = serde_json::to_string(&s).unwrap();
        let s2: State = serde_json::from_str(&json).unwrap();
        assert_eq!(s, s2);
    }

    #[test]
    fn trace_serialization() {
        let t = Trace {
            states: vec![State {
                timestep: 0,
                variables: vec![(1, true)],
                description: "x=T".to_string(),
            }],
        };
        let json = serde_json::to_string(&t).unwrap();
        let t2: Trace = serde_json::from_str(&json).unwrap();
        assert_eq!(t, t2);
    }

    #[test]
    fn partial_assignment_pads_false() {
        // Assignment shorter than num_steps * num_vars
        let assignment = vec![true];
        let trace = extract_trace(&assignment, 1, &make_var_names(&["x"]));
        assert_eq!(trace.states[0].variables[0], (1, true));
    }
}
