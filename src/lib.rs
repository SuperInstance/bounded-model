//! # bounded-model
//!
//! Bounded model checking with a simple DPLL SAT solver.
//!
//! This crate implements bounded model checking (BMC), a technique for finding
//! bugs in finite-state systems by unrolling the transition relation for a
//! bounded number of steps and checking satisfiability with a SAT solver.
//!
//! ## Modules
//!
//! - [`cnf`] — CNF formula representation
//! - [`solver`] — DPLL SAT solver
//! - [`encoding`] — State transition system → CNF encoding
//! - [`bound`] — Bounded model checking loop
//! - [`counterexample`] — Counterexample extraction
//! - [`system`] — Transition system definition

pub mod bound;
pub mod cnf;
pub mod counterexample;
pub mod encoding;
pub mod solver;
pub mod system;
