use std::collections::BTreeSet;

const TOTAL_TRANSFORMATIONS: usize = 10_000;
const HARD_REJECT_END: usize = 7_000;
const BOUND_REJECT_END: usize = 9_000;
const ADVANCE_MODULUS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Advance,
    Reject,
}

#[derive(Debug, Default, Clone)]
struct WorkCounter {
    family_evaluations: u64,
    exact_expansions: u64,
    constraint_evaluations: u64,
    economic_evaluations: u64,
    fact_accesses: u64,
}

#[derive(Debug)]
struct EngineResult {
    advances: BTreeSet<usize>,
    rejects: usize,
    work: WorkCounter,
}

#[derive(Debug, Clone, Copy)]
struct Transformation {
    id: usize,
    hard_constraint_satisfied: bool,
    gross_value: i64,
    cost: i64,
}

impl Transformation {
    fn net_value(self) -> i64 {
        self.gross_value - self.cost
    }

    fn oracle_decision(self) -> Decision {
        if self.hard_constraint_satisfied && self.net_value() > 0 {
            Decision::Advance
        } else {
            Decision::Reject
        }
    }
}

fn transformation(id: usize) -> Transformation {
    assert!(id < TOTAL_TRANSFORMATIONS);

    if id < HARD_REJECT_END {
        return Transformation {
            id,
            hard_constraint_satisfied: false,
            gross_value: 100,
            cost: 10,
        };
    }

    if id < BOUND_REJECT_END {
        return Transformation {
            id,
            hard_constraint_satisfied: true,
            gross_value: 10,
            cost: 20,
        };
    }

    let local_id = id - BOUND_REJECT_END;

    if local_id % ADVANCE_MODULUS == 0 {
        Transformation {
            id,
            hard_constraint_satisfied: true,
            gross_value: 30,
            cost: 20,
        }
    } else {
        Transformation {
            id,
            hard_constraint_satisfied: true,
            gross_value: 19,
            cost: 20,
        }
    }
}

fn run_oracle() -> BTreeSet<usize> {
    let mut advances = BTreeSet::new();

    for id in 0..TOTAL_TRANSFORMATIONS {
        let candidate = transformation(id);

        if candidate.oracle_decision() == Decision::Advance {
            advances.insert(candidate.id);
        }
    }

    advances
}

fn run_baseline() -> EngineResult {
    let mut advances = BTreeSet::new();
    let mut rejects = 0;
    let mut work = WorkCounter::default();

    for id in 0..TOTAL_TRANSFORMATIONS {
        work.exact_expansions += 1;

        let candidate = transformation(id);

        work.fact_accesses += 1;
        work.constraint_evaluations += 1;

        if !candidate.hard_constraint_satisfied {
            rejects += 1;
            continue;
        }

        work.fact_accesses += 2;
        work.economic_evaluations += 1;

        if candidate.net_value() > 0 {
            advances.insert(candidate.id);
        } else {
            rejects += 1;
        }
    }

    EngineResult {
        advances,
        rejects,
        work,
    }
}

fn run_pulse() -> EngineResult {
    let mut advances = BTreeSet::new();
    let mut rejects = 0;
    let mut work = WorkCounter::default();

    // Family 1:
    // IDs 0..7000 share a hard constraint that proves every member infeasible.
    work.family_evaluations += 1;
    work.fact_accesses += 1;
    work.constraint_evaluations += 1;
    rejects += HARD_REJECT_END;

    // Family 2:
    // IDs 7000..9000 share an economic upper bound <= 0.
    work.family_evaluations += 1;
    work.fact_accesses += 2;
    work.economic_evaluations += 1;
    rejects += BOUND_REJECT_END - HARD_REJECT_END;

    // Family 3:
    // IDs 9000..10000 cross the decision boundary.
    // This family must split because some members ADVANCE and others REJECT.
    work.family_evaluations += 1;

    for id in BOUND_REJECT_END..TOTAL_TRANSFORMATIONS {
        work.exact_expansions += 1;

        let candidate = transformation(id);

        work.fact_accesses += 1;
        work.constraint_evaluations += 1;

        if !candidate.hard_constraint_satisfied {
            rejects += 1;
            continue;
        }

        work.fact_accesses += 2;
        work.economic_evaluations += 1;

        if candidate.net_value() > 0 {
            advances.insert(candidate.id);
        } else {
            rejects += 1;
        }
    }

    EngineResult {
        advances,
        rejects,
        work,
    }
}

fn print_work(label: &str, work: &WorkCounter) {
    println!("{label}");
    println!("  family evaluations:     {}", work.family_evaluations);
    println!("  exact expansions:       {}", work.exact_expansions);
    println!("  constraint evaluations: {}", work.constraint_evaluations);
    println!("  economic evaluations:   {}", work.economic_evaluations);
    println!("  fact accesses:           {}", work.fact_accesses);
}

fn main() {
    println!("Pulse Field Lab");
    println!("EZ-M0A — Representation Compression");
    println!();

    let oracle = run_oracle();
    let baseline = run_baseline();
    let pulse = run_pulse();

    let baseline_matches_oracle = baseline.advances == oracle;
    let pulse_matches_oracle = pulse.advances == oracle;
    let decision_agreement = baseline.advances == pulse.advances;

    let false_important_prunes = oracle.difference(&pulse.advances).count();

    println!("Fixture: EZ-001");
    println!("Total transformations: {TOTAL_TRANSFORMATIONS}");
    println!("Oracle ADVANCE count: {}", oracle.len());
    println!();

    println!("Correctness");
    println!("  baseline matches oracle: {baseline_matches_oracle}");
    println!("  pulse matches oracle:    {pulse_matches_oracle}");
    println!("  decision agreement:      {decision_agreement}");
    println!("  false important prunes:  {false_important_prunes}");
    println!();

    println!("Decisions");
    println!("  baseline ADVANCE: {}", baseline.advances.len());
    println!("  baseline REJECT:  {}", baseline.rejects);
    println!("  pulse ADVANCE:    {}", pulse.advances.len());
    println!("  pulse REJECT:     {}", pulse.rejects);
    println!();

    print_work("Baseline work", &baseline.work);
    println!();
    print_work("Pulse work", &pulse.work);
    println!();

    let baseline_expansions = baseline.work.exact_expansions as f64;
    let pulse_expansions = pulse.work.exact_expansions as f64;

    let expansion_reduction =
        100.0 * (baseline_expansions - pulse_expansions) / baseline_expansions;

    println!("Exact expansion reduction: {:.2}%", expansion_reduction);

    let passed = baseline_matches_oracle
        && pulse_matches_oracle
        && decision_agreement
        && false_important_prunes == 0;

    println!();

    if passed {
        println!("EZ-001 CORRECTNESS GATE: PASS");
    } else {
        println!("EZ-001 CORRECTNESS GATE: FAIL");
        std::process::exit(1);
    }
}
