use std::collections::BTreeSet;

const TRANSFORMATION_COUNT: usize = 100;
const MIN_QUANTITY: usize = 1;
const MAX_QUANTITY: usize = 100;

const NOVELTY_REGION_START: usize = 20;
const NOVELTY_REGION_END: usize = 30;
const NOVELTY_ACTIVE_SPLIT: usize = 25;
const SENSITIVE_REGION_START: usize = 50;
const SENSITIVE_REGION_END: usize = 75;

const SAFE_BASE_EDGE: i64 = 10;
const SENSITIVE_BASE_EDGE: i64 = 24;
const IMPACT: i64 = 1;
const FIXED_COST: i64 = 100;

const SAFE_UNKNOWN_MIN: i64 = -2;
const SAFE_UNKNOWN_MAX: i64 = 2;
const NOVELTY_UNKNOWN_MAX: i64 = 20;
const SENSITIVE_UNKNOWN_MIN: i64 = -5;
const SENSITIVE_UNKNOWN_MAX: i64 = 5;

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
struct World {
    novelty_generation: u64,
    novelty_active: bool,
}

#[derive(Debug, Clone, Copy)]
struct CandidateFamily {
    id_start: usize,
    id_end: usize,
    quantity_start: usize,
    quantity_end: usize,
}

impl CandidateFamily {
    fn point_count(self) -> usize {
        (self.id_end - self.id_start) * (self.quantity_end - self.quantity_start)
    }

    fn is_empty(self) -> bool {
        self.id_start >= self.id_end || self.quantity_start >= self.quantity_end
    }
}

#[derive(Debug, Clone, Copy)]
struct SafeIgnoranceCertificate {
    id_start: usize,
    id_end: usize,
    dependency_generation: u64,
    decision: Decision,
}

#[derive(Debug, Default, Clone)]
struct WorkCounter {
    family_evaluations: u64,
    family_splits: u64,
    semantic_splits: u64,
    family_rejections: u64,
    exact_expansions: u64,
    economic_evaluations: u64,
    bound_evaluations: u64,
    fact_accesses: u64,
    certificate_checks: u64,
    certificates_issued: u64,
    certificates_reused: u64,
    certificates_invalidated: u64,
    reactivated_points: u64,
}

#[derive(Debug)]
struct EngineResult {
    advances: BTreeSet<CandidateKey>,
    rejects: usize,
    work: WorkCounter,
}

#[derive(Debug)]
struct UnsafeResult {
    advances: BTreeSet<CandidateKey>,
    rejects: usize,
    exact_expansions: u64,
    stale_suppressed_points: usize,
}

fn phase_a_world() -> World {
    World {
        novelty_generation: 1,
        novelty_active: false,
    }
}

fn phase_b_world() -> World {
    World {
        novelty_generation: 2,
        novelty_active: true,
    }
}

fn in_region(transformation_id: usize, start: usize, end: usize) -> bool {
    transformation_id >= start && transformation_id < end
}

fn in_novelty_region(transformation_id: usize) -> bool {
    in_region(
        transformation_id,
        NOVELTY_REGION_START,
        NOVELTY_REGION_END,
    )
}

fn in_sensitive_region(transformation_id: usize) -> bool {
    in_region(
        transformation_id,
        SENSITIVE_REGION_START,
        SENSITIVE_REGION_END,
    )
}

fn base_edge(transformation_id: usize) -> i64 {
    if in_sensitive_region(transformation_id) {
        SENSITIVE_BASE_EDGE
    } else {
        SAFE_BASE_EDGE
    }
}

fn actual_unknown_adjustment(transformation_id: usize, world: &World) -> i64 {
    if world.novelty_active && in_novelty_region(transformation_id) {
        if transformation_id < NOVELTY_ACTIVE_SPLIT {
            NOVELTY_UNKNOWN_MAX
        } else {
            0
        }
    } else if in_sensitive_region(transformation_id) {
        match transformation_id % 3 {
            0 => -4,
            1 => 0,
            _ => 4,
        }
    } else {
        0
    }
}

fn unknown_bounds_for_family(family: CandidateFamily, world: &World) -> (i64, i64) {
    if family.id_start >= SENSITIVE_REGION_START && family.id_end <= SENSITIVE_REGION_END {
        (SENSITIVE_UNKNOWN_MIN, SENSITIVE_UNKNOWN_MAX)
    } else if family.id_start >= NOVELTY_REGION_START && family.id_end <= NOVELTY_REGION_END {
        let upper = if world.novelty_active {
            NOVELTY_UNKNOWN_MAX
        } else {
            SAFE_UNKNOWN_MAX
        };

        (SAFE_UNKNOWN_MIN, upper)
    } else {
        (SAFE_UNKNOWN_MIN, SAFE_UNKNOWN_MAX)
    }
}

fn family_base_edge(family: CandidateFamily) -> i64 {
    if family.id_start >= SENSITIVE_REGION_START && family.id_end <= SENSITIVE_REGION_END {
        SENSITIVE_BASE_EDGE
    } else {
        SAFE_BASE_EDGE
    }
}

fn profit_with_adjustment(
    transformation_id: usize,
    quantity: usize,
    unknown_adjustment: i64,
) -> i64 {
    let quantity = quantity as i64;
    let edge = base_edge(transformation_id) + unknown_adjustment;

    quantity * edge - IMPACT * quantity * quantity - FIXED_COST
}

fn oracle_decision(transformation_id: usize, quantity: usize, world: &World) -> Decision {
    let adjustment = actual_unknown_adjustment(transformation_id, world);
    let profit = profit_with_adjustment(transformation_id, quantity, adjustment);

    if profit > 0 {
        Decision::Advance
    } else {
        Decision::Reject
    }
}

fn evaluate_exact(
    transformation_id: usize,
    quantity: usize,
    world: &World,
    work: &mut WorkCounter,
) -> Decision {
    work.exact_expansions += 1;
    work.economic_evaluations += 1;
    work.fact_accesses += 4;

    oracle_decision(transformation_id, quantity, world)
}

fn run_oracle(world: &World) -> BTreeSet<CandidateKey> {
    let mut advances = BTreeSet::new();

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            if oracle_decision(transformation_id, quantity, world) == Decision::Advance {
                advances.insert(CandidateKey {
                    transformation_id,
                    quantity,
                });
            }
        }
    }

    advances
}

fn run_eager(world: &World) -> EngineResult {
    let mut result = EngineResult {
        advances: BTreeSet::new(),
        rejects: 0,
        work: WorkCounter::default(),
    };

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
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

    result
}

fn optimistic_profit_at_quantity(quantity: usize, edge: i64) -> i64 {
    let quantity = quantity as i64;

    quantity * edge - IMPACT * quantity * quantity - FIXED_COST
}

fn family_optimistic_profit_upper_bound(
    family: CandidateFamily,
    world: &World,
    work: &mut WorkCounter,
) -> i64 {
    work.family_evaluations += 1;
    work.economic_evaluations += 1;
    work.bound_evaluations += 1;
    work.fact_accesses += 3;

    let (_, unknown_max) = unknown_bounds_for_family(family, world);
    let edge = family_base_edge(family) + unknown_max;
    let first_quantity = family.quantity_start;
    let last_quantity = family.quantity_end - 1;
    let vertex_quantity = if edge > 0 {
        (edge / (2 * IMPACT)).max(1) as usize
    } else {
        first_quantity
    };
    let bounded_vertex = vertex_quantity.clamp(first_quantity, last_quantity);

    let first_profit = optimistic_profit_at_quantity(first_quantity, edge);
    let last_profit = optimistic_profit_at_quantity(last_quantity, edge);
    let vertex_profit = optimistic_profit_at_quantity(bounded_vertex, edge);

    first_profit.max(last_profit).max(vertex_profit)
}

fn family_proven_reject(
    family: CandidateFamily,
    world: &World,
    work: &mut WorkCounter,
) -> bool {
    family_optimistic_profit_upper_bound(family, world, work) <= 0
}

fn family_crosses_boundary(family: CandidateFamily, boundary: usize) -> bool {
    family.id_start < boundary && family.id_end > boundary
}

fn next_semantic_boundary(family: CandidateFamily) -> Option<usize> {
    [
        NOVELTY_REGION_START,
        NOVELTY_REGION_END,
        SENSITIVE_REGION_START,
        SENSITIVE_REGION_END,
    ]
    .into_iter()
    .find(|boundary| family_crosses_boundary(family, *boundary))
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

fn is_exact_novelty_family(family: CandidateFamily) -> bool {
    family.id_start == NOVELTY_REGION_START && family.id_end == NOVELTY_REGION_END
}

fn issue_phase_a_certificate() -> SafeIgnoranceCertificate {
    let world = phase_a_world();
    let family = CandidateFamily {
        id_start: NOVELTY_REGION_START,
        id_end: NOVELTY_REGION_END,
        quantity_start: MIN_QUANTITY,
        quantity_end: MAX_QUANTITY + 1,
    };
    let mut work = WorkCounter::default();

    assert!(family_proven_reject(family, &world, &mut work));

    SafeIgnoranceCertificate {
        id_start: family.id_start,
        id_end: family.id_end,
        dependency_generation: world.novelty_generation,
        decision: Decision::Reject,
    }
}

fn cached_certificate_matches_family(
    certificate: SafeIgnoranceCertificate,
    family: CandidateFamily,
) -> bool {
    certificate.id_start == family.id_start && certificate.id_end == family.id_end
}

fn resolve_exact_family(family: CandidateFamily, world: &World, result: &mut EngineResult) {
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

fn resolve_family(
    family: CandidateFamily,
    world: &World,
    cached_certificate: SafeIgnoranceCertificate,
    result: &mut EngineResult,
) {
    if family.is_empty() {
        return;
    }

    if let Some(boundary) = next_semantic_boundary(family) {
        result.work.family_splits += 1;
        result.work.semantic_splits += 1;

        let (left, right) = split_family_at_id(family, boundary);

        resolve_family(left, world, cached_certificate, result);
        resolve_family(right, world, cached_certificate, result);
        return;
    }

    if is_exact_novelty_family(family)
        && cached_certificate_matches_family(cached_certificate, family)
    {
        result.work.certificate_checks += 1;
        result.work.fact_accesses += 1;

        if cached_certificate.dependency_generation == world.novelty_generation
            && cached_certificate.decision == Decision::Reject
        {
            result.rejects += family.point_count();
            result.work.family_rejections += 1;
            result.work.certificates_reused += 1;
            return;
        }

        result.work.certificates_invalidated += 1;
        result.work.reactivated_points += family.point_count() as u64;
    }

    result.work.certificate_checks += 1;

    if family_proven_reject(family, world, &mut result.work) {
        result.rejects += family.point_count();
        result.work.family_rejections += 1;
        result.work.certificates_issued += 1;
        return;
    }

    resolve_exact_family(family, world, result);
}

fn run_pulse(
    world: &World,
    cached_certificate: SafeIgnoranceCertificate,
) -> EngineResult {
    let mut result = EngineResult {
        advances: BTreeSet::new(),
        rejects: 0,
        work: WorkCounter::default(),
    };

    let root = CandidateFamily {
        id_start: 0,
        id_end: TRANSFORMATION_COUNT,
        quantity_start: MIN_QUANTITY,
        quantity_end: MAX_QUANTITY + 1,
    };

    resolve_family(root, world, cached_certificate, &mut result);

    result
}

fn run_unsafe_stale_suppression(
    world: &World,
    stale_certificate: SafeIgnoranceCertificate,
) -> UnsafeResult {
    let mut advances = BTreeSet::new();
    let mut rejects = 0;
    let mut exact_expansions = 0;

    for transformation_id in 0..TRANSFORMATION_COUNT {
        if transformation_id >= stale_certificate.id_start
            && transformation_id < stale_certificate.id_end
            && stale_certificate.decision == Decision::Reject
        {
            rejects += MAX_QUANTITY - MIN_QUANTITY + 1;
            continue;
        }

        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            exact_expansions += 1;
            let key = CandidateKey {
                transformation_id,
                quantity,
            };

            match oracle_decision(transformation_id, quantity, world) {
                Decision::Advance => {
                    advances.insert(key);
                }
                Decision::Reject => {
                    rejects += 1;
                }
            }
        }
    }

    UnsafeResult {
        advances,
        rejects,
        exact_expansions,
        stale_suppressed_points: (stale_certificate.id_end - stale_certificate.id_start)
            * (MAX_QUANTITY - MIN_QUANTITY + 1),
    }
}

fn novelty_region_advance_count(world: &World) -> usize {
    let mut count = 0;

    for transformation_id in NOVELTY_REGION_START..NOVELTY_REGION_END {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            if oracle_decision(transformation_id, quantity, world) == Decision::Advance {
                count += 1;
            }
        }
    }

    count
}

fn phase_a_certificate_is_valid(certificate: SafeIgnoranceCertificate) -> bool {
    let world = phase_a_world();
    let family = CandidateFamily {
        id_start: certificate.id_start,
        id_end: certificate.id_end,
        quantity_start: MIN_QUANTITY,
        quantity_end: MAX_QUANTITY + 1,
    };
    let mut work = WorkCounter::default();

    cached_certificate_matches_family(certificate, family)
        && certificate.dependency_generation == world.novelty_generation
        && certificate.decision == Decision::Reject
        && family_proven_reject(family, &world, &mut work)
}

fn novelty_break_is_decision_relevant() -> bool {
    let phase_a = phase_a_world();
    let phase_b = phase_b_world();

    novelty_region_advance_count(&phase_a) == 0 && novelty_region_advance_count(&phase_b) > 0
}

fn print_work(label: &str, work: &WorkCounter) {
    println!("{label}");
    println!("  family evaluations:       {}", work.family_evaluations);
    println!("  family splits:            {}", work.family_splits);
    println!("  semantic splits:          {}", work.semantic_splits);
    println!("  family rejections:        {}", work.family_rejections);
    println!("  exact expansions:         {}", work.exact_expansions);
    println!(
        "  economic evaluations:     {}",
        work.economic_evaluations
    );
    println!("  bound evaluations:        {}", work.bound_evaluations);
    println!("  fact accesses:            {}", work.fact_accesses);
    println!("  certificate checks:       {}", work.certificate_checks);
    println!("  certificates issued:      {}", work.certificates_issued);
    println!("  certificates reused:      {}", work.certificates_reused);
    println!(
        "  certificates invalidated: {}",
        work.certificates_invalidated
    );
    println!(
        "  reactivated points:       {}",
        work.reactivated_points
    );
}

fn main() {
    println!("Pulse Field Lab");
    println!("EZ-M0A — Compression Without Loss");
    println!();

    let phase_a = phase_a_world();
    let phase_b = phase_b_world();
    let certificate = issue_phase_a_certificate();

    let oracle_a = run_oracle(&phase_a);
    let oracle_b = run_oracle(&phase_b);
    let eager_b = run_eager(&phase_b);
    let unsafe_b = run_unsafe_stale_suppression(&phase_b, certificate);
    let pulse_b = run_pulse(&phase_b, certificate);

    let total_candidate_states = TRANSFORMATION_COUNT * (MAX_QUANTITY - MIN_QUANTITY + 1);
    let phase_a_certificate_valid = phase_a_certificate_is_valid(certificate);
    let novelty_break_relevant = novelty_break_is_decision_relevant();
    let novelty_advances_a = novelty_region_advance_count(&phase_a);
    let novelty_advances_b = novelty_region_advance_count(&phase_b);

    let unsafe_false_suppressions = oracle_b.difference(&unsafe_b.advances).count();
    let unsafe_false_advances = unsafe_b.advances.difference(&oracle_b).count();

    let eager_matches_oracle = eager_b.advances == oracle_b;
    let pulse_matches_oracle = pulse_b.advances == oracle_b;
    let decision_agreement = eager_b.advances == pulse_b.advances;
    let false_suppressions = oracle_b.difference(&pulse_b.advances).count();
    let false_advances = pulse_b.advances.difference(&oracle_b).count();

    println!("Fixture: EZ-008 — Safe Ignorance");
    println!("Total candidate states: {total_candidate_states}");
    println!(
        "Novelty region: {}..{}",
        NOVELTY_REGION_START, NOVELTY_REGION_END
    );
    println!(
        "Sensitive region: {}..{}",
        SENSITIVE_REGION_START, SENSITIVE_REGION_END
    );
    println!(
        "Phase A novelty generation: {}",
        phase_a.novelty_generation
    );
    println!(
        "Phase B novelty generation: {}",
        phase_b.novelty_generation
    );
    println!("Phase A Safe Ignorance certificate valid: {phase_a_certificate_valid}");
    println!("Novelty break is decision relevant: {novelty_break_relevant}");
    println!("Phase A novelty-region ADVANCE count: {novelty_advances_a}");
    println!("Phase B novelty-region ADVANCE count: {novelty_advances_b}");
    println!("Phase A Oracle ADVANCE count: {}", oracle_a.len());
    println!("Phase B Oracle ADVANCE count: {}", oracle_b.len());
    println!();

    println!("Unsafe stale-certificate suppression");
    println!(
        "  stale-suppressed points: {}",
        unsafe_b.stale_suppressed_points
    );
    println!("  exact expansions:        {}", unsafe_b.exact_expansions);
    println!("  ADVANCE:                 {}", unsafe_b.advances.len());
    println!("  REJECT:                  {}", unsafe_b.rejects);
    println!(
        "  execution-relevant false suppression: {unsafe_false_suppressions}"
    );
    println!("  false advances:                      {unsafe_false_advances}");
    println!();

    println!("Correctness");
    println!("  eager matches oracle: {eager_matches_oracle}");
    println!("  pulse matches oracle: {pulse_matches_oracle}");
    println!("  decision agreement:   {decision_agreement}");
    println!("  false suppression:    {false_suppressions}");
    println!("  false advances:       {false_advances}");
    println!(
        "  certificates invalidated: {}",
        pulse_b.work.certificates_invalidated
    );
    println!(
        "  reactivated points:       {}",
        pulse_b.work.reactivated_points
    );
    println!();

    println!("Decisions");
    println!("  eager ADVANCE: {}", eager_b.advances.len());
    println!("  eager REJECT:  {}", eager_b.rejects);
    println!("  pulse ADVANCE: {}", pulse_b.advances.len());
    println!("  pulse REJECT:  {}", pulse_b.rejects);
    println!();

    print_work("Eager work", &eager_b.work);
    println!();
    print_work("Pulse work", &pulse_b.work);
    println!();

    let eager_expansions = eager_b.work.exact_expansions as f64;
    let pulse_expansions = pulse_b.work.exact_expansions as f64;
    let expansion_reduction =
        100.0 * (eager_expansions - pulse_expansions) / eager_expansions;

    println!("Exact expansion reduction: {:.2}%", expansion_reduction);

    let passed = phase_a_certificate_valid
        && novelty_break_relevant
        && unsafe_false_suppressions > 0
        && unsafe_false_advances == 0
        && eager_matches_oracle
        && pulse_matches_oracle
        && decision_agreement
        && false_suppressions == 0
        && false_advances == 0
        && eager_b.rejects == pulse_b.rejects
        && pulse_b.work.certificates_invalidated > 0
        && pulse_b.work.reactivated_points > 0
        && pulse_b.work.certificates_issued > 0
        && pulse_b.work.exact_expansions < eager_b.work.exact_expansions;

    println!();

    if passed {
        println!("EZ-008 CORRECTNESS GATE: PASS");
    } else {
        println!("EZ-008 CORRECTNESS GATE: FAIL");
        std::process::exit(1);
    }
}
