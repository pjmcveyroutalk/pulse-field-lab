use std::collections::BTreeSet;

const TRANSFORMATION_COUNT: usize = 100;
const MIN_QUANTITY: usize = 1;
const MAX_QUANTITY: usize = 100;

const BLOCKED_REGION_START: usize = 30;
const BLOCKED_REGION_END: usize = 60;

const MIN_EDGE: i64 = 24;
const MAX_EDGE: i64 = 30;
const MIN_IMPACT: i64 = 1;
const FIXED_COST: i64 = 20;

const EXACT_FAMILY_POINT_LIMIT: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Advance,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CandidateKey {
    transformation_id: usize,
    quantity: usize,
}

#[derive(Debug, Clone, Copy)]
struct VisibleWorld {
    transformation_count: usize,
    min_quantity: usize,
    max_quantity: usize,
    blocked_region_start: usize,
    blocked_region_end: usize,
}

#[derive(Debug, Clone, Copy)]
struct OracleCandidate {
    key: CandidateKey,
    hard_feasible: bool,
    gross_value: i64,
    cost: i64,
}

impl OracleCandidate {
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
    family_splits: u64,
    family_rejections: u64,
    exact_expansions: u64,
    constraint_evaluations: u64,
    economic_evaluations: u64,
    fact_accesses: u64,
    bound_evaluations: u64,
}

#[derive(Debug)]
struct EngineResult {
    advances: BTreeSet<CandidateKey>,
    rejects: usize,
    work: WorkCounter,
}

#[derive(Debug, Clone, Copy)]
struct CandidateFamily {
    id_start: usize,
    id_end: usize,
    quantity_start: usize,
    quantity_end: usize,
}

impl CandidateFamily {
    fn id_len(self) -> usize {
        self.id_end - self.id_start
    }

    fn quantity_len(self) -> usize {
        self.quantity_end - self.quantity_start
    }

    fn point_count(self) -> usize {
        self.id_len() * self.quantity_len()
    }

    fn is_empty(self) -> bool {
        self.id_start >= self.id_end || self.quantity_start >= self.quantity_end
    }
}

fn visible_world() -> VisibleWorld {
    VisibleWorld {
        transformation_count: TRANSFORMATION_COUNT,
        min_quantity: MIN_QUANTITY,
        max_quantity: MAX_QUANTITY,
        blocked_region_start: BLOCKED_REGION_START,
        blocked_region_end: BLOCKED_REGION_END,
    }
}

fn transformation_edge(transformation_id: usize) -> i64 {
    MIN_EDGE + (transformation_id % 7) as i64
}

fn transformation_impact(transformation_id: usize) -> i64 {
    MIN_IMPACT + (transformation_id % 3) as i64
}

fn candidate_is_blocked(transformation_id: usize, world: &VisibleWorld) -> bool {
    transformation_id >= world.blocked_region_start && transformation_id < world.blocked_region_end
}

fn candidate_profit(transformation_id: usize, quantity: usize) -> i64 {
    let quantity = quantity as i64;
    let edge = transformation_edge(transformation_id);
    let impact = transformation_impact(transformation_id);

    quantity * edge - impact * quantity * quantity - FIXED_COST
}

fn oracle_candidate(
    transformation_id: usize,
    quantity: usize,
    world: &VisibleWorld,
) -> OracleCandidate {
    assert!(transformation_id < world.transformation_count);
    assert!(quantity >= world.min_quantity);
    assert!(quantity <= world.max_quantity);

    let hard_feasible = !candidate_is_blocked(transformation_id, world);
    let edge = transformation_edge(transformation_id);
    let impact = transformation_impact(transformation_id);
    let quantity_i64 = quantity as i64;

    let gross_value = quantity_i64 * edge;
    let cost = impact * quantity_i64 * quantity_i64 + FIXED_COST;

    OracleCandidate {
        key: CandidateKey {
            transformation_id,
            quantity,
        },
        hard_feasible,
        gross_value,
        cost,
    }
}

fn evaluate_exact(
    transformation_id: usize,
    quantity: usize,
    world: &VisibleWorld,
    work: &mut WorkCounter,
) -> Decision {
    work.exact_expansions += 1;
    work.constraint_evaluations += 1;
    work.fact_accesses += 2;

    if candidate_is_blocked(transformation_id, world) {
        return Decision::Reject;
    }

    work.economic_evaluations += 1;
    work.fact_accesses += 4;

    if candidate_profit(transformation_id, quantity) > 0 {
        Decision::Advance
    } else {
        Decision::Reject
    }
}

fn run_oracle(world: &VisibleWorld) -> BTreeSet<CandidateKey> {
    let mut advances = BTreeSet::new();

    for transformation_id in 0..world.transformation_count {
        for quantity in world.min_quantity..=world.max_quantity {
            let candidate = oracle_candidate(transformation_id, quantity, world);

            if candidate.decision() == Decision::Advance {
                advances.insert(candidate.key);
            }
        }
    }

    advances
}

fn run_baseline(world: &VisibleWorld) -> EngineResult {
    let mut advances = BTreeSet::new();
    let mut rejects = 0;
    let mut work = WorkCounter::default();

    for transformation_id in 0..world.transformation_count {
        for quantity in world.min_quantity..=world.max_quantity {
            let key = CandidateKey {
                transformation_id,
                quantity,
            };

            match evaluate_exact(transformation_id, quantity, world, &mut work) {
                Decision::Advance => {
                    advances.insert(key);
                }
                Decision::Reject => {
                    rejects += 1;
                }
            }
        }
    }

    EngineResult {
        advances,
        rejects,
        work,
    }
}

fn family_fully_blocked(
    family: CandidateFamily,
    world: &VisibleWorld,
    work: &mut WorkCounter,
) -> bool {
    work.family_evaluations += 1;
    work.constraint_evaluations += 1;
    work.fact_accesses += 2;

    family.id_start >= world.blocked_region_start && family.id_end <= world.blocked_region_end
}

fn optimistic_profit_at_quantity(quantity: usize) -> i64 {
    let quantity = quantity as i64;

    quantity * MAX_EDGE - MIN_IMPACT * quantity * quantity - FIXED_COST
}

fn clamp_quantity(quantity: usize, start: usize, end: usize) -> usize {
    quantity.clamp(start, end)
}

fn family_optimistic_profit_upper_bound(family: CandidateFamily, work: &mut WorkCounter) -> i64 {
    work.family_evaluations += 1;
    work.economic_evaluations += 1;
    work.bound_evaluations += 1;
    work.fact_accesses += 3;

    let first_quantity = family.quantity_start;
    let last_quantity = family.quantity_end - 1;

    let vertex_quantity = (MAX_EDGE / (2 * MIN_IMPACT)) as usize;
    let bounded_vertex = clamp_quantity(vertex_quantity, first_quantity, last_quantity);

    let first_profit = optimistic_profit_at_quantity(first_quantity);
    let last_profit = optimistic_profit_at_quantity(last_quantity);
    let vertex_profit = optimistic_profit_at_quantity(bounded_vertex);

    first_profit.max(last_profit).max(vertex_profit)
}

fn family_proven_economic_reject(family: CandidateFamily, work: &mut WorkCounter) -> bool {
    family_optimistic_profit_upper_bound(family, work) <= 0
}

fn split_family(family: CandidateFamily) -> (CandidateFamily, CandidateFamily) {
    if family.id_len() >= family.quantity_len() && family.id_len() > 1 {
        let midpoint = family.id_start + family.id_len() / 2;

        (
            CandidateFamily {
                id_start: family.id_start,
                id_end: midpoint,
                quantity_start: family.quantity_start,
                quantity_end: family.quantity_end,
            },
            CandidateFamily {
                id_start: midpoint,
                id_end: family.id_end,
                quantity_start: family.quantity_start,
                quantity_end: family.quantity_end,
            },
        )
    } else {
        let midpoint = family.quantity_start + family.quantity_len() / 2;

        (
            CandidateFamily {
                id_start: family.id_start,
                id_end: family.id_end,
                quantity_start: family.quantity_start,
                quantity_end: midpoint,
            },
            CandidateFamily {
                id_start: family.id_start,
                id_end: family.id_end,
                quantity_start: midpoint,
                quantity_end: family.quantity_end,
            },
        )
    }
}

fn resolve_exact_family(family: CandidateFamily, world: &VisibleWorld, result: &mut EngineResult) {
    for transformation_id in family.id_start..family.id_end {
        for quantity in family.quantity_start..family.quantity_end {
            let key = CandidateKey {
                transformation_id,
                quantity,
            };

            match evaluate_exact(transformation_id, quantity, world, &mut result.work) {
                Decision::Advance => {
                    result.advances.insert(key);
                }
                Decision::Reject => {
                    result.rejects += 1;
                }
            }
        }
    }
}

fn resolve_family(family: CandidateFamily, world: &VisibleWorld, result: &mut EngineResult) {
    if family.is_empty() {
        return;
    }

    if family_fully_blocked(family, world, &mut result.work) {
        result.rejects += family.point_count();
        result.work.family_rejections += 1;
        return;
    }

    if family_proven_economic_reject(family, &mut result.work) {
        result.rejects += family.point_count();
        result.work.family_rejections += 1;
        return;
    }

    if family.point_count() <= EXACT_FAMILY_POINT_LIMIT {
        resolve_exact_family(family, world, result);
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
        work: WorkCounter::default(),
    };

    let root = CandidateFamily {
        id_start: 0,
        id_end: world.transformation_count,
        quantity_start: world.min_quantity,
        quantity_end: world.max_quantity + 1,
    };

    resolve_family(root, world, &mut result);

    result
}

fn quantity_coupling_exists(oracle: &BTreeSet<CandidateKey>, world: &VisibleWorld) -> bool {
    for transformation_id in 0..world.transformation_count {
        if candidate_is_blocked(transformation_id, world) {
            continue;
        }

        let mut has_advance = false;
        let mut has_reject = false;

        for quantity in world.min_quantity..=world.max_quantity {
            let key = CandidateKey {
                transformation_id,
                quantity,
            };

            if oracle.contains(&key) {
                has_advance = true;
            } else {
                has_reject = true;
            }

            if has_advance && has_reject {
                return true;
            }
        }
    }

    false
}

fn print_work(label: &str, work: &WorkCounter) {
    println!("{label}");
    println!("  family evaluations:     {}", work.family_evaluations);
    println!("  family splits:          {}", work.family_splits);
    println!("  family rejections:      {}", work.family_rejections);
    println!("  exact expansions:       {}", work.exact_expansions);
    println!("  constraint evaluations: {}", work.constraint_evaluations);
    println!("  economic evaluations:   {}", work.economic_evaluations);
    println!("  bound evaluations:      {}", work.bound_evaluations);
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

    let total_candidate_states =
        world.transformation_count * (world.max_quantity - world.min_quantity + 1);

    let baseline_matches_oracle = baseline.advances == oracle;
    let pulse_matches_oracle = pulse.advances == oracle;
    let decision_agreement = baseline.advances == pulse.advances;
    let false_important_prunes = oracle.difference(&pulse.advances).count();
    let coupling_confirmed = quantity_coupling_exists(&oracle, &world);

    println!("Fixture: EZ-003 — Quantity Coupling");
    println!("Transformations: {}", world.transformation_count);
    println!(
        "Quantity range: {}..={}",
        world.min_quantity, world.max_quantity
    );
    println!("Total candidate states: {total_candidate_states}");
    println!("Oracle ADVANCE count: {}", oracle.len());
    println!("Quantity coupling present: {coupling_confirmed}");
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

    let passed = coupling_confirmed
        && baseline_matches_oracle
        && pulse_matches_oracle
        && decision_agreement
        && false_important_prunes == 0
        && baseline.rejects == pulse.rejects;

    println!();

    if passed {
        println!("EZ-003 CORRECTNESS GATE: PASS");
    } else {
        println!("EZ-003 CORRECTNESS GATE: FAIL");
        std::process::exit(1);
    }
}
