use std::collections::BTreeSet;

const TRANSFORMATION_COUNT: usize = 100;
const MIN_QUANTITY: usize = 1;
const MAX_QUANTITY: usize = 100;

const MERGE_REGION_START: usize = 20;
const MERGE_REGION_END: usize = 30;
const HIDDEN_CAPABILITY_BOUNDARY: usize = 25;

const MIN_EDGE: i64 = 24;
const MAX_EDGE: i64 = 30;
const MIN_IMPACT: i64 = 1;
const MAX_IMPACT: i64 = 3;
const FIXED_COST: i64 = 20;

const MERGE_REGION_EDGE_ADJUSTMENT: i64 = 20;
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
struct Evidence {
    edge_adjustment: i64,
}

#[derive(Debug, Clone, Copy)]
struct VisibleWorld {
    transformation_count: usize,
    min_quantity: usize,
    max_quantity: usize,
    merge_region_start: usize,
    merge_region_end: usize,
    hidden_capability_boundary: usize,
}

#[derive(Debug, Clone, Copy)]
struct OracleCandidate {
    key: CandidateKey,
    execution_enabled: bool,
    gross_value: i64,
    cost: i64,
}

impl OracleCandidate {
    fn decision(self) -> Decision {
        if self.execution_enabled && self.gross_value - self.cost > 0 {
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
    semantic_splits: u64,
    family_rejections: u64,
    capability_family_rejections: u64,
    exact_expansions: u64,
    economic_evaluations: u64,
    fact_accesses: u64,
    bound_evaluations: u64,
    compression_checks: u64,
    dangerous_merges_refused: u64,
}

#[derive(Debug)]
struct EngineResult {
    advances: BTreeSet<CandidateKey>,
    rejects: usize,
    work: WorkCounter,
}

#[derive(Debug)]
struct UnsafeCoarseResult {
    advances: BTreeSet<CandidateKey>,
    rejects: usize,
    exact_expansions: u64,
    merged_points: usize,
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
        merge_region_start: MERGE_REGION_START,
        merge_region_end: MERGE_REGION_END,
        hidden_capability_boundary: HIDDEN_CAPABILITY_BOUNDARY,
    }
}

fn transformation_in_merge_region(transformation_id: usize, world: &VisibleWorld) -> bool {
    transformation_id >= world.merge_region_start && transformation_id < world.merge_region_end
}

fn transformation_execution_enabled(transformation_id: usize, world: &VisibleWorld) -> bool {
    if transformation_in_merge_region(transformation_id, world) {
        transformation_id < world.hidden_capability_boundary
    } else {
        true
    }
}

fn transformation_edge(transformation_id: usize) -> i64 {
    MIN_EDGE + (transformation_id % 7) as i64
}

fn transformation_impact(transformation_id: usize) -> i64 {
    MIN_IMPACT + (transformation_id % 3) as i64
}

fn current_evidence_for_candidate(transformation_id: usize, world: &VisibleWorld) -> Evidence {
    Evidence {
        edge_adjustment: if transformation_in_merge_region(transformation_id, world) {
            MERGE_REGION_EDGE_ADJUSTMENT
        } else {
            0
        },
    }
}

fn candidate_profit(transformation_id: usize, quantity: usize, evidence: Evidence) -> i64 {
    let quantity = quantity as i64;
    let edge = transformation_edge(transformation_id) + evidence.edge_adjustment;
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

    let evidence = current_evidence_for_candidate(transformation_id, world);
    let quantity_i64 = quantity as i64;
    let edge = transformation_edge(transformation_id) + evidence.edge_adjustment;
    let impact = transformation_impact(transformation_id);

    OracleCandidate {
        key: CandidateKey {
            transformation_id,
            quantity,
        },
        execution_enabled: transformation_execution_enabled(transformation_id, world),
        gross_value: quantity_i64 * edge,
        cost: impact * quantity_i64 * quantity_i64 + FIXED_COST,
    }
}

fn evaluate_exact(
    transformation_id: usize,
    quantity: usize,
    world: &VisibleWorld,
    work: &mut WorkCounter,
) -> Decision {
    work.exact_expansions += 1;
    work.economic_evaluations += 1;
    work.fact_accesses += 5;

    oracle_candidate(transformation_id, quantity, world).decision()
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

fn run_unsafe_coarse_merge(world: &VisibleWorld) -> UnsafeCoarseResult {
    let mut advances = BTreeSet::new();
    let mut rejects = 0;
    let mut work = WorkCounter::default();

    for transformation_id in 0..world.transformation_count {
        if transformation_in_merge_region(transformation_id, world) {
            continue;
        }

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

    let merged_points = (world.merge_region_end - world.merge_region_start)
        * (world.max_quantity - world.min_quantity + 1);

    rejects += merged_points;

    UnsafeCoarseResult {
        advances,
        rejects,
        exact_expansions: work.exact_expansions,
        merged_points,
    }
}

fn family_crosses_id_boundary(family: CandidateFamily, boundary: usize) -> bool {
    family.id_start < boundary && family.id_end > boundary
}

fn family_crosses_merge_start(family: CandidateFamily, world: &VisibleWorld) -> bool {
    family_crosses_id_boundary(family, world.merge_region_start)
}

fn family_crosses_merge_end(family: CandidateFamily, world: &VisibleWorld) -> bool {
    family_crosses_id_boundary(family, world.merge_region_end)
}

fn family_crosses_hidden_capability_boundary(
    family: CandidateFamily,
    world: &VisibleWorld,
) -> bool {
    family_crosses_id_boundary(family, world.hidden_capability_boundary)
}

fn family_fully_in_merge_region(family: CandidateFamily, world: &VisibleWorld) -> bool {
    family.id_start >= world.merge_region_start && family.id_end <= world.merge_region_end
}

fn family_fully_in_disabled_region(family: CandidateFamily, world: &VisibleWorld) -> bool {
    family.id_start >= world.hidden_capability_boundary && family.id_end <= world.merge_region_end
}

fn family_evidence(family: CandidateFamily, world: &VisibleWorld) -> Evidence {
    Evidence {
        edge_adjustment: if family_fully_in_merge_region(family, world) {
            MERGE_REGION_EDGE_ADJUSTMENT
        } else {
            0
        },
    }
}

fn optimistic_profit_at_quantity(quantity: usize, evidence: Evidence) -> i64 {
    let quantity = quantity as i64;
    let edge = MAX_EDGE + evidence.edge_adjustment;

    quantity * edge - MIN_IMPACT * quantity * quantity - FIXED_COST
}

fn family_optimistic_profit_upper_bound(
    family: CandidateFamily,
    evidence: Evidence,
    work: &mut WorkCounter,
) -> i64 {
    work.family_evaluations += 1;
    work.economic_evaluations += 1;
    work.bound_evaluations += 1;
    work.fact_accesses += 4;

    let first_quantity = family.quantity_start;
    let last_quantity = family.quantity_end - 1;
    let optimistic_edge = MAX_EDGE + evidence.edge_adjustment;

    let vertex_quantity = if optimistic_edge > 0 {
        (optimistic_edge / (2 * MIN_IMPACT)).max(1) as usize
    } else {
        first_quantity
    };

    let bounded_vertex = vertex_quantity.clamp(first_quantity, last_quantity);

    let first_profit = optimistic_profit_at_quantity(first_quantity, evidence);
    let last_profit = optimistic_profit_at_quantity(last_quantity, evidence);
    let vertex_profit = optimistic_profit_at_quantity(bounded_vertex, evidence);

    first_profit.max(last_profit).max(vertex_profit)
}

fn family_proven_reject(
    family: CandidateFamily,
    evidence: Evidence,
    work: &mut WorkCounter,
) -> bool {
    family_optimistic_profit_upper_bound(family, evidence, work) <= 0
}

fn split_family_at_id(
    family: CandidateFamily,
    boundary: usize,
) -> (CandidateFamily, CandidateFamily) {
    assert!(family.id_start < boundary);
    assert!(boundary < family.id_end);

    (
        CandidateFamily {
            id_start: family.id_start,
            id_end: boundary,
            quantity_start: family.quantity_start,
            quantity_end: family.quantity_end,
        },
        CandidateFamily {
            id_start: boundary,
            id_end: family.id_end,
            quantity_start: family.quantity_start,
            quantity_end: family.quantity_end,
        },
    )
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

fn compression_certificate_allows_merge(
    family: CandidateFamily,
    world: &VisibleWorld,
    work: &mut WorkCounter,
) -> bool {
    work.compression_checks += 1;
    work.fact_accesses += 2;

    if family_fully_in_merge_region(family, world)
        && family_crosses_hidden_capability_boundary(family, world)
    {
        work.dangerous_merges_refused += 1;
        false
    } else {
        true
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

    if family_crosses_merge_start(family, world) {
        result.work.family_splits += 1;
        result.work.semantic_splits += 1;

        let (left, right) = split_family_at_id(family, world.merge_region_start);

        resolve_family(left, world, result);
        resolve_family(right, world, result);
        return;
    }

    if family_crosses_merge_end(family, world) {
        result.work.family_splits += 1;
        result.work.semantic_splits += 1;

        let (left, right) = split_family_at_id(family, world.merge_region_end);

        resolve_family(left, world, result);
        resolve_family(right, world, result);
        return;
    }

    if !compression_certificate_allows_merge(family, world, &mut result.work) {
        result.work.family_splits += 1;
        result.work.semantic_splits += 1;

        let (left, right) = split_family_at_id(family, world.hidden_capability_boundary);

        resolve_family(left, world, result);
        resolve_family(right, world, result);
        return;
    }

    if family_fully_in_disabled_region(family, world) {
        result.rejects += family.point_count();
        result.work.family_rejections += 1;
        result.work.capability_family_rejections += 1;
        return;
    }

    let evidence = family_evidence(family, world);

    if family_proven_reject(family, evidence, &mut result.work) {
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

fn coarse_signature_collapses_merge_region(world: &VisibleWorld) -> bool {
    let first = current_evidence_for_candidate(world.merge_region_start, world);
    let last = current_evidence_for_candidate(world.merge_region_end - 1, world);

    let first_capability = transformation_execution_enabled(world.merge_region_start, world);
    let last_capability = transformation_execution_enabled(world.merge_region_end - 1, world);

    first.edge_adjustment == last.edge_adjustment && first_capability != last_capability
}

fn enabled_merge_region_advance_count(world: &VisibleWorld) -> usize {
    let mut count = 0;

    for transformation_id in world.merge_region_start..world.hidden_capability_boundary {
        for quantity in world.min_quantity..=world.max_quantity {
            if oracle_candidate(transformation_id, quantity, world).decision() == Decision::Advance
            {
                count += 1;
            }
        }
    }

    count
}

fn disabled_positive_economics_count(world: &VisibleWorld) -> usize {
    let mut count = 0;

    for transformation_id in world.hidden_capability_boundary..world.merge_region_end {
        let evidence = current_evidence_for_candidate(transformation_id, world);

        for quantity in world.min_quantity..=world.max_quantity {
            if candidate_profit(transformation_id, quantity, evidence) > 0 {
                count += 1;
            }
        }
    }

    count
}

fn dangerous_false_merge_fixture_is_valid(world: &VisibleWorld) -> bool {
    world.merge_region_start < world.hidden_capability_boundary
        && world.hidden_capability_boundary < world.merge_region_end
        && coarse_signature_collapses_merge_region(world)
        && enabled_merge_region_advance_count(world) > 0
        && disabled_positive_economics_count(world) > 0
}

fn print_work(label: &str, work: &WorkCounter) {
    println!("{label}");
    println!(
        "  family evaluations:             {}",
        work.family_evaluations
    );
    println!("  family splits:                  {}", work.family_splits);
    println!("  semantic splits:                {}", work.semantic_splits);
    println!(
        "  family rejections:              {}",
        work.family_rejections
    );
    println!(
        "  capability family rejections:   {}",
        work.capability_family_rejections
    );
    println!(
        "  exact expansions:               {}",
        work.exact_expansions
    );
    println!(
        "  economic evaluations:           {}",
        work.economic_evaluations
    );
    println!(
        "  bound evaluations:              {}",
        work.bound_evaluations
    );
    println!("  fact accesses:                   {}", work.fact_accesses);
    println!(
        "  compression checks:             {}",
        work.compression_checks
    );
    println!(
        "  dangerous merges refused:       {}",
        work.dangerous_merges_refused
    );
}

fn main() {
    println!("Pulse Field Lab");
    println!("EZ-M0A — Representation Compression");
    println!();

    let world = visible_world();
    let oracle = run_oracle(&world);
    let baseline = run_baseline(&world);
    let unsafe_coarse = run_unsafe_coarse_merge(&world);
    let pulse = run_pulse(&world);

    let total_candidate_states =
        world.transformation_count * (world.max_quantity - world.min_quantity + 1);

    let fixture_valid = dangerous_false_merge_fixture_is_valid(&world);

    let enabled_merge_advances = enabled_merge_region_advance_count(&world);

    let disabled_positive_economics = disabled_positive_economics_count(&world);

    let unsafe_false_prunes = oracle.difference(&unsafe_coarse.advances).count();

    let unsafe_false_advances = unsafe_coarse.advances.difference(&oracle).count();

    let baseline_matches_oracle = baseline.advances == oracle;
    let pulse_matches_oracle = pulse.advances == oracle;
    let decision_agreement = baseline.advances == pulse.advances;

    let false_important_prunes = oracle.difference(&pulse.advances).count();

    let false_advances = pulse.advances.difference(&oracle).count();

    println!("Fixture: EZ-007 — Dangerous False Merge");
    println!("Total candidate states: {total_candidate_states}");
    println!(
        "Merge region: {}..{}",
        world.merge_region_start, world.merge_region_end
    );
    println!(
        "Hidden capability boundary: {}",
        world.hidden_capability_boundary
    );
    println!(
        "Coarse signature collapses merge region: {}",
        coarse_signature_collapses_merge_region(&world)
    );
    println!("Enabled-side Oracle ADVANCE count: {enabled_merge_advances}");
    println!(
        "Disabled-side positive economics count: \
         {disabled_positive_economics}"
    );
    println!("Dangerous false-merge fixture valid: {fixture_valid}");
    println!("Oracle ADVANCE count: {}", oracle.len());
    println!();

    println!("Unsafe coarse merger");
    println!(
        "  merged points without distinction: {}",
        unsafe_coarse.merged_points
    );
    println!(
        "  exact expansions:                 {}",
        unsafe_coarse.exact_expansions
    );
    println!(
        "  ADVANCE:                          {}",
        unsafe_coarse.advances.len()
    );
    println!(
        "  REJECT:                           {}",
        unsafe_coarse.rejects
    );
    println!("  economically important false prunes: {unsafe_false_prunes}");
    println!("  false advances:                      {unsafe_false_advances}");
    println!();

    println!("Correctness");
    println!("  baseline matches oracle: {baseline_matches_oracle}");
    println!("  pulse matches oracle:    {pulse_matches_oracle}");
    println!("  decision agreement:      {decision_agreement}");
    println!("  false important prunes:  {false_important_prunes}");
    println!("  false advances:          {false_advances}");
    println!(
        "  dangerous merges refused: {}",
        pulse.work.dangerous_merges_refused
    );
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

    let passed = fixture_valid
        && unsafe_false_prunes > 0
        && unsafe_false_advances == 0
        && pulse.work.dangerous_merges_refused > 0
        && pulse.work.capability_family_rejections > 0
        && baseline_matches_oracle
        && pulse_matches_oracle
        && decision_agreement
        && false_important_prunes == 0
        && false_advances == 0
        && baseline.rejects == pulse.rejects
        && pulse.work.exact_expansions < baseline.work.exact_expansions;

    println!();

    if passed {
        println!("EZ-007 CORRECTNESS GATE: PASS");
    } else {
        println!("EZ-007 CORRECTNESS GATE: FAIL");
        std::process::exit(1);
    }
}
