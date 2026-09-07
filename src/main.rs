use std::collections::BTreeSet;

const TOTAL_TRANSFORMATIONS: usize = 10_000;
const ADVANCE_MODULUS: usize = 37;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Advance,
    Reject,
    Unresolved,
}

#[derive(Debug, Clone, Copy)]
struct VisibleFacts {
    blocked_region_start: usize,
    blocked_region_end: usize,
    expensive_modulus: usize,
    expensive_remainder: usize,
    weak_region_start: usize,
    weak_region_end: usize,
}

#[derive(Debug)]
struct VisibleWorld {
    total_transformations: usize,
    facts: VisibleFacts,
}

#[derive(Debug, Clone, Copy)]
struct OracleTransformation {
    id: usize,
    hard_feasible: bool,
    gross_value: i64,
    cost: i64,
}

impl OracleTransformation {
    fn decision(self) -> Decision {
        if !self.hard_feasible {
            Decision::Reject
        } else if self.gross_value - self.cost > 0 {
            Decision::Advance
        } else {
            Decision::Reject
        }
    }
}

#[derive(Debug, Default, Clone)]
struct WorkCounter {
    family_evaluations: u64,
    exact_expansions: u64,
    constraint_evaluations: u64,
    economic_evaluations: u64,
    fact_accesses: u64,
    family_splits: u64,
    family_rejections: u64,
}

#[derive(Debug)]
struct EngineResult {
    advances: BTreeSet<usize>,
    rejects: usize,
    unresolved: usize,
    work: WorkCounter,
}

#[derive(Debug, Clone, Copy)]
struct CandidateFamily {
    start: usize,
    end: usize,
}

impl CandidateFamily {
    fn len(self) -> usize {
        self.end - self.start
    }

    fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

fn visible_world() -> VisibleWorld {
    VisibleWorld {
        total_transformations: TOTAL_TRANSFORMATIONS,
        facts: VisibleFacts {
            blocked_region_start: 1_500,
            blocked_region_end: 6_500,
            expensive_modulus: 5,
            expensive_remainder: 2,
            weak_region_start: 4_000,
            weak_region_end: 9_000,
        },
    }
}

fn oracle_transformation(id: usize, world: &VisibleWorld) -> OracleTransformation {
    assert!(id < world.total_transformations);

    let facts = world.facts;

    let blocked = id >= facts.blocked_region_start && id < facts.blocked_region_end;

    let expensive = id % facts.expensive_modulus == facts.expensive_remainder;

    let weak = id >= facts.weak_region_start && id < facts.weak_region_end;

    let gross_value = if id % ADVANCE_MODULUS == 0 {
        40
    } else if weak {
        19
    } else {
        25
    };

    let cost = if expensive { 30 } else { 20 };

    OracleTransformation {
        id,
        hard_feasible: !blocked,
        gross_value,
        cost,
    }
}

fn evaluate_exact(id: usize, world: &VisibleWorld, work: &mut WorkCounter) -> Decision {
    work.exact_expansions += 1;

    let facts = world.facts;

    work.fact_accesses += 2;
    work.constraint_evaluations += 1;

    let blocked = id >= facts.blocked_region_start && id < facts.blocked_region_end;

    if blocked {
        return Decision::Reject;
    }

    work.fact_accesses += 2;
    work.economic_evaluations += 1;

    let expensive = id % facts.expensive_modulus == facts.expensive_remainder;

    let weak = id >= facts.weak_region_start && id < facts.weak_region_end;

    let gross_value = if id % ADVANCE_MODULUS == 0 {
        40
    } else if weak {
        19
    } else {
        25
    };

    let cost = if expensive { 30 } else { 20 };

    if gross_value - cost > 0 {
        Decision::Advance
    } else {
        Decision::Reject
    }
}

fn run_oracle(world: &VisibleWorld) -> BTreeSet<usize> {
    let mut advances = BTreeSet::new();

    for id in 0..world.total_transformations {
        let candidate = oracle_transformation(id, world);

        if candidate.decision() == Decision::Advance {
            advances.insert(candidate.id);
        }
    }

    advances
}

fn run_baseline(world: &VisibleWorld) -> EngineResult {
    let mut advances = BTreeSet::new();
    let mut rejects = 0;
    let mut unresolved = 0;
    let mut work = WorkCounter::default();

    for id in 0..world.total_transformations {
        match evaluate_exact(id, world, &mut work) {
            Decision::Advance => {
                advances.insert(id);
            }
            Decision::Reject => {
                rejects += 1;
            }
            Decision::Unresolved => {
                unresolved += 1;
            }
        }
    }

    EngineResult {
        advances,
        rejects,
        unresolved,
        work,
    }
}

fn family_fully_blocked(
    family: CandidateFamily,
    world: &VisibleWorld,
    work: &mut WorkCounter,
) -> bool {
    work.family_evaluations += 1;
    work.fact_accesses += 2;
    work.constraint_evaluations += 1;

    family.start >= world.facts.blocked_region_start && family.end <= world.facts.blocked_region_end
}

fn family_crosses_block_boundary(family: CandidateFamily, world: &VisibleWorld) -> bool {
    let facts = world.facts;

    let crosses_start =
        family.start < facts.blocked_region_start && family.end > facts.blocked_region_start;

    let crosses_end =
        family.start < facts.blocked_region_end && family.end > facts.blocked_region_end;

    crosses_start || crosses_end
}

fn family_economically_uniform_reject(
    family: CandidateFamily,
    world: &VisibleWorld,
    work: &mut WorkCounter,
) -> bool {
    work.family_evaluations += 1;
    work.fact_accesses += 4;
    work.economic_evaluations += 1;

    let facts = world.facts;

    let fully_weak = family.start >= facts.weak_region_start && family.end <= facts.weak_region_end;

    if !fully_weak {
        return false;
    }

    let contains_advance_exception = (family.start..family.end).any(|id| id % ADVANCE_MODULUS == 0);

    if contains_advance_exception {
        return false;
    }

    true
}

fn split_family(family: CandidateFamily) -> (CandidateFamily, CandidateFamily) {
    let midpoint = family.start + family.len() / 2;

    (
        CandidateFamily {
            start: family.start,
            end: midpoint,
        },
        CandidateFamily {
            start: midpoint,
            end: family.end,
        },
    )
}

fn resolve_family(family: CandidateFamily, world: &VisibleWorld, result: &mut EngineResult) {
    if family.is_empty() {
        return;
    }

    if family_fully_blocked(family, world, &mut result.work) {
        result.rejects += family.len();
        result.work.family_rejections += 1;
        return;
    }

    if family_crosses_block_boundary(family, world) && family.len() > 1 {
        result.work.family_splits += 1;
        let (left, right) = split_family(family);
        resolve_family(left, world, result);
        resolve_family(right, world, result);
        return;
    }

    if family_economically_uniform_reject(family, world, &mut result.work) {
        result.rejects += family.len();
        result.work.family_rejections += 1;
        return;
    }

    if family.len() <= 32 {
        for id in family.start..family.end {
            match evaluate_exact(id, world, &mut result.work) {
                Decision::Advance => {
                    result.advances.insert(id);
                }
                Decision::Reject => {
                    result.rejects += 1;
                }
                Decision::Unresolved => {
                    result.unresolved += 1;
                }
            }
        }

        return;
    }

    result.work.family_splits += 1;
    let (left, right) = split_family(family);
    resolve_family(left, world, result);
    resolve_family(right, world, result);
}

fn run_pulse(world: &VisibleWorld) -> EngineResult {
    let mut result = EngineResult {
        advances: BTreeSet::new(),
        rejects: 0,
        unresolved: 0,
        work: WorkCounter::default(),
    };

    let root = CandidateFamily {
        start: 0,
        end: world.total_transformations,
    };

    resolve_family(root, world, &mut result);

    result
}

fn print_work(label: &str, work: &WorkCounter) {
    println!("{label}");
    println!("  family evaluations:     {}", work.family_evaluations);
    println!("  family splits:          {}", work.family_splits);
    println!("  family rejections:      {}", work.family_rejections);
    println!("  exact expansions:       {}", work.exact_expansions);
    println!("  constraint evaluations: {}", work.constraint_evaluations);
    println!("  economic evaluations:   {}", work.economic_evaluations);
    println!("  fact accesses:           {}", work.fact_accesses);
}

fn main() {
    println!("Pulse Field Lab");
    println!("EZ-M0A — Representation Compression");
    println!();

    let world = visible_world();
    let oracle = run_oracle(&world);
    let baseline = run_baseline(&world);
    let pulse = run_pulse(&world);

    let baseline_matches_oracle = baseline.advances == oracle;
    let pulse_matches_oracle = pulse.advances == oracle;
    let decision_agreement = baseline.advances == pulse.advances;
    let false_important_prunes = oracle.difference(&pulse.advances).count();

    println!("Fixture: EZ-002 — Overlapping Constraints");
    println!("Total transformations: {}", world.total_transformations);
    println!("Oracle ADVANCE count: {}", oracle.len());
    println!();

    println!("Correctness");
    println!("  baseline matches oracle: {baseline_matches_oracle}");
    println!("  pulse matches oracle:    {pulse_matches_oracle}");
    println!("  decision agreement:      {decision_agreement}");
    println!("  false important prunes:  {false_important_prunes}");
    println!("  baseline unresolved:     {}", baseline.unresolved);
    println!("  pulse unresolved:        {}", pulse.unresolved);
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
        && false_important_prunes == 0
        && baseline.unresolved == 0
        && pulse.unresolved == 0;

    println!();

    if passed {
        println!("EZ-002 CORRECTNESS GATE: PASS");
    } else {
        println!("EZ-002 CORRECTNESS GATE: FAIL");
        std::process::exit(1);
    }
}
