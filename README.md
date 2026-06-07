# bounded-model

**Bounded model checking with a simple DPLL SAT solver — in pure Rust.**

```
┌─────────────────────────────────────────────────────────┐
│                   bounded-model                         │
│                                                         │
│   Transition System                                     │
│         │                                               │
│         ▼                                               │
│   ┌───────────┐     ┌──────────┐     ┌──────────────┐  │
│   │  system   │────▶│ encoding │────▶│  CNF Formula │  │
│   └───────────┘     └──────────┘     └──────┬───────┘  │
│                                             │           │
│                              ┌──────────────▼────────┐  │
│                              │   DPLL SAT Solver     │  │
│                              │  · Unit Propagation   │  │
│                              │  · Pure Literal Elim  │  │
│                              │  · Backtracking       │  │
│                              └──────────┬────────────┘  │
│                                         │               │
│                              ┌──────────▼────────────┐  │
│                              │  Bounded Checker      │  │
│                              │  k=1, k=2, k=3, ...  │  │
│                              └──────────┬────────────┘  │
│                                         │               │
│                              ┌──────────▼────────────┐  │
│                              │  Counterexample       │  │
│                              │  Trace Extraction     │  │
│                              └───────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

## What is Bounded Model Checking?

Bounded model checking (BMC) is a formal verification technique that searches
for bugs in finite-state systems by unrolling the transition relation for a
bounded number of steps. At each step count *k*, the system encodes:

1. **Initial state** — where the system starts
2. **Transition relation** — how the system evolves, unrolled *k* times
3. **Negated property** — "does something bad happen by step *k*?"

This produces a propositional formula in conjunctive normal form (CNF), which
is handed to a SAT solver. If the solver finds the formula satisfiable, the
satisfying assignment is a **counterexample** — a concrete execution trace that
reaches a bad state within *k* steps.

### Why "Bounded"?

Traditional model checking explores all reachable states, which can be
astronomically large (the infamous *state explosion problem*). BMC sidesteps
this by only looking *k* steps deep. This makes it:

- **Effective at finding shallow bugs** quickly
- **Scalable** — the encoding grows linearly with *k*
- **Sound but not complete** — if no bug is found within *k* steps, there
  might still be a bug at *k+1*. You can increase *k*, but you can't prove
  the absence of bugs with BMC alone (for that, you'd need *k*-induction or
  interpolation).

## Theoretical Background

### SAT Encoding

The core idea behind BMC is that we can represent the execution of a
finite-state system as a propositional formula. Given a system with *n*
boolean state variables and a bound *k*, we create *k+1* copies of the state
variables:

```
s₀₀, s₀₁, ..., s₀ₙ₋₁   ← state at timestep 0
s₁₀, s₁₁, ..., s₁ₙ₋₁   ← state at timestep 1
...
sₖ₀, sₖ₁, ..., sₖₙ₋₁   ← state at timestep k
```

The BMC formula is:

```
I(s₀) ∧ T(s₀, s₁) ∧ T(s₁, s₂) ∧ ... ∧ T(sₖ₋₁, sₖ) ∧ ¬P(sₖ)
```

Where:
- **I(s₀)** constrains the initial state
- **T(sᵢ, sᵢ₊₁)** is the transition relation between consecutive states
- **¬P(sₖ)** is the negation of the property (we want to find violations)

The formula is satisfiable if and only if there exists an execution of length
*k* that starts in an initial state and violates the property.

### The DPLL Algorithm

The SAT solver in this crate implements the Davis-Putnam-Logemann-Loveland
(DPLL) algorithm, one of the foundational algorithms for Boolean satisfiability.
DPLL works by:

1. **Unit Propagation**: If a clause has exactly one unassigned literal (a
   *unit clause*), that literal must be true. This forces the variable's
   assignment and may create new unit clauses — a cascade of deductions.

2. **Pure Literal Elimination**: If a variable appears with only one polarity
   across all clauses (e.g., only positive occurrences), it can be assigned
   to satisfy all those clauses without conflict.

3. **Branching**: When no more deductions are possible, pick an unassigned
   variable, try assigning it to `true`, and recurse. If that fails, try
   `false`. If both fail, backtrack.

The recursion depth is bounded by the number of variables (typically < 100
in BMC encodings for small systems), making it safe from stack overflow.

### k-Induction

While BMC alone can only find bugs, *k*-induction extends the approach to
prove properties. The idea:

- **Base case**: Show the property holds for the first *k* steps (BMC with
  the property instead of its negation).
- **Inductive step**: Assume the property holds for *k* consecutive steps,
  then show it must hold for step *k+1*.

This crate focuses on the BMC part (bug finding). *k*-induction would be a
natural extension.

## Module Overview

| Module | Description | Key Types |
|--------|-------------|-----------|
| `cnf` | CNF formula representation | `CnfFormula` |
| `solver` | DPLL SAT solver | `SatResult`, `solve()` |
| `encoding` | Transition system → CNF | `TransitionEncoding` |
| `bound` | BMC loop | `BmcResult`, `BmcConfig` |
| `counterexample` | Trace extraction | `Trace`, `State` |
| `system` | Transition system definition | `TransitionSystem`, `PropertyFn` |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bounded-model = "0.1"
```

## Examples

### Example 1: Basic SAT Solving

Solve a simple CNF formula directly:

```rust
use bounded_model::cnf::CnfFormula;
use bounded_model::solver;

// Build formula: (x₁ ∨ x₂) ∧ (¬x₁ ∨ x₂) ∧ (x₁ ∨ ¬x₂)
let formula = CnfFormula::new()
    .add_clause(vec![1, 2])
    .add_clause(vec![-1, 2])
    .add_clause(vec![1, -2]);

let result = solver::solve(&formula);

if result.sat {
    println!("Satisfiable!");
    let assignment = result.assignment.unwrap();
    for (i, val) in assignment.iter().enumerate() {
        println!("  x{} = {}", i + 1, if *val { "T" } else { "F" });
    }
    println!("Decisions: {}, Propagations: {}",
        result.num_decisions, result.num_propagations);
} else {
    println!("Unsatisfiable.");
}
```

### Example 2: Bounded Model Checking with Encoding

Encode a simple state machine and check for property violations:

```rust
use bounded_model::encoding::TransitionEncoding;
use bounded_model::bound::{BmcConfig, check};
use bounded_model::counterexample;

// 2 boolean state variables: x (var 1), y (var 2)
let encoding = TransitionEncoding::new(2)
    // Initial state: x=T, y=F
    .initial(vec![vec![1], vec![-2]])
    // Transition: if x then x' ∨ y', always y → ¬y'
    .transition(vec![
        vec![-1, 3],    // x → x'
        vec![-1, 4],    // x → y'
        vec![2, -4],    // y → ¬y'
    ])
    // Negated property: "x is never false" → we check if x can be false
    .negated_property(vec![vec![-3]]); // ¬x' at final step

let config = BmcConfig::new(10);
let result = check(&encoding, &config);

if result.sat {
    println!("Counterexample found at k={}", result.k);
    if let Some(assignment) = &result.assignment {
        let var_names = vec!["x".to_string(), "y".to_string()];
        let trace = counterexample::extract_trace(
            assignment, 2, &var_names
        );
        println!("{}", counterexample::format_trace(&trace));
    }
} else {
    println!("No violation found within {} steps", config.max_k);
}
```

### Example 3: Full Pipeline with TransitionSystem

Define a named transition system, encode it, and search for bugs:

```rust
use bounded_model::system::{TransitionSystem, PropertyFn};
use bounded_model::bound::{BmcConfig, check};
use bounded_model::counterexample;

let mut builder = TransitionSystem::builder();

// Define states
let idle = builder.add_state("idle");
let active = builder.add_state("active");
let fault = builder.add_state("fault");
let recovery = builder.add_state("recovery");

// Set initial state
builder.add_initial(idle);

// Define transitions
builder.add_transition(idle, active);
builder.add_transition(active, active);
builder.add_transition(active, fault);
builder.add_transition(fault, recovery);
builder.add_transition(recovery, active);

// Property: "fault" state should never be reached
builder.property(PropertyFn::NeverReach(fault));

let system = builder.build();

// Encode and check
let encoding = system.to_encoding();
let config = BmcConfig::new(5);
let result = check(&encoding, &config);

match result.sat {
    true => {
        println!("⚠ Bug found! Fault reachable at k={}", result.k);
        if let Some(asgn) = &result.assignment {
            let trace = counterexample::extract_trace(
                asgn,
                system.num_states(),
                &system.states,
            );
            println!("{}", counterexample::format_trace(&trace));
        }
    }
    false => {
        println!("✓ No fault reachable within {} steps", config.max_k);
    }
}
```

## Design Decisions

### Why DPLL instead of CDCL?

Modern SAT solvers use Conflict-Driven Clause Learning (CDCL), which adds
learned clauses during search to avoid revisiting conflicts. CDCL solvers
like MiniSat, Glucose, or CaDiCaL can handle millions of clauses.

This crate implements plain DPLL for educational clarity. The algorithm is
easy to understand, verify, and extend. For real-world BMC on large systems,
you'd want to swap in a CDCL solver via a trait or feature flag.

### Why `i32` Literals?

DIMACS CNF format uses signed integers for literals. This is the simplest
representation — no newtypes, no enums, just `+v` and `-v`. It maps directly
to what SAT solvers consume.

### Why Zero Dependencies (Except serde)?

This crate is designed to be self-contained and auditable. The only external
dependency is `serde` for serialization of all public types. The SAT solver,
encoding logic, and BMC loop are all implemented from scratch.

### Variable Encoding

State variable `i` at timestep `t` is encoded as CNF variable:

```
var = i + t × num_state_vars + 1
```

This gives a compact, predictable encoding that's easy to reverse when
extracting counterexamples.

## Performance Characteristics

| Aspect | Characteristic |
|--------|---------------|
| Solver | DPLL (exponential worst case, PSPACE-complete) |
| BMC encoding size | O(k × |transition_relation|) clauses |
| Max recursion depth | Number of variables (bounded) |
| Typical use case | Small systems, educational, prototyping |
| Not suitable for | Industrial-scale verification (use a CDCL solver) |

## Limitations

1. **No proof of correctness**: BMC can only find bugs up to bound *k*. It
   cannot prove properties hold for all reachable states.
2. **No clause learning**: The DPLL solver doesn't learn from conflicts, so
   it may revisit the same failed search branches.
3. **No incremental solving**: Each BMC step re-solves from scratch. Real BMC
   tools reuse solver state across iterations.
4. **One-hot state encoding**: The `TransitionSystem::to_encoding()` method
   uses one variable per state, which is exponentially larger than binary
   encoding for systems with many states.

## References

1. **Biere, A., Cimatti, A., Clarke, E.M., & Zhu, Y.** (1999).
   *Symbolic Model Checking without BDDs.* Tools and Algorithms for the
   Construction and Analysis of Systems (TACAS 1999), LNCS 1579, pp. 193–207.
   Springer. — **The original BMC paper.**

2. **Davis, M., Logemann, G., & Loveland, D.** (1962).
   *A Machine Program for Theorem-Proving.* Communications of the ACM, 5(7),
   pp. 394–397. — **The DPLL algorithm.**

3. **Clarke, E.M., Grumberg, O., & Peled, D.A.** (1999).
   *Model Checking.* MIT Press. — **The definitive textbook on model checking.**

4. **Kroening, D., & Strichman, O.** (2008).
   *Decision Procedures: An Algorithmic Point of View.* Springer. — **Covers
   SAT solving, BMC encoding, and related decision procedures.**

5. **Biere, A., Heule, M., van Maaren, H., & Walsh, T.** (2009).
   *Handbook of Satisfiability.* IOS Press. — **Comprehensive reference on
   SAT solving, including CDCL and practical considerations.**

6. **Sheeran, M., Singh, S., & Stålmarck, G.** (2000).
   *Checking Safety Properties Using Induction and a SAT-Solver.* Formal
   Methods in Computer-Aided Design (FMCAD 2000), LNCS 1954, pp. 127–144.
   — **k-induction for safety properties.**

7. **Eén, N., & Sörensson, N.** (2003).
   *An Extensible SAT-solver.* Theory and Applications of Satisfiability
   Testing (SAT 2003), LNCS 2919, pp. 502–518. — **MiniSat, the basis for
   most modern SAT solvers.**

## License

MIT
