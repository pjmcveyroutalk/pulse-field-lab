use std::collections::BTreeSet;

const TRANSFORMATION_COUNT: usize = 100;
const MIN_QUANTITY: usize = 1;
const MAX_QUANTITY: usize = 100;

const TIME_REGION_START: usize = 20;
const TIME_REGION_END: usize = 30;

const CACHED_OBSERVED_SLOT: u64 = 100;
const EXECUTION_DEADLINE_SLOT: u64 = 120;
const CURRENT_PROTOCOL_SLOT: u64 = 140;

const MARKET_DEPENDENCY_ID: u64 = 1;
const ACCOUNT_DIGEST: u64 = 0xA11CE;

const MIN_EDGE: i64 = 24;
const MAX_EDGE: i64 = 30;
const MIN_IMPACT: i64 = 1;
const MAX_IMPACT: i64 = 3;
const FIXED_COST: i64 = 20;

const TIME_REGION_EDGE_ADJUSTMENT: i64 = 20;
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
    dependency_id: u64,
    account_digest: u64,
    observed_slot: u64,
    valid_until_slot: u64,
    edge_adjustment: i64,
}

#[derive(Debug, Clone, Copy)]
struct VisibleWorld {
    transformation_count: usize,
    min_quantity: usize,
    max_quantity: usize,
    time_region_start: usize,
    time_region_end: usize,
    dependency_id: u64,
    account_digest: u64,
    current_protocol_slot: u64,
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
    family_rejections: u64,
    family_advances: u64,
    exact_expansions: u64,
    economic_evaluations: u64,
    fact_accesses: u64,
    bound_evaluations: u64,
    proof_checks: u64,
    expired_proofs_blocked: u64,
    expired_advance_proofs_blocked: u64,
    byte_only_false_accepts: u64,
    fresh_proof_authorizations: u64,
    stale_authorized_decisions: u64,
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
        time_region_start: TIME_REGION_START,
        time_region_end: TIME_REGION_END,
        dependency_id: MARKET_DEPENDENCY_ID,
        account_digest: ACCOUNT_DIGEST,
        current_protocol_slot: CURRENT_PROTOCOL_SLOT,
    }
}

fn transformation_in_time_region(transformation_id: usize, world: &VisibleWorld) -> bool {
    transformation_id >= world.time_region_start && transformation_id < world.time_region_end
}

fn transformation_edge(transformation_id: usize) -> i64 {
    MIN_EDGE + (transformation_id % 7) as i64
}

fn transformation_impact(transformation_id: usize) -> i64 {
    MIN_IMPACT + (transformation_id % 3) as i64
}

fn protocol_capability_enabled(
    transformation_id: usize,
    protocol_slot: u64,
    world: &VisibleWorld,
) -> bool {
    if transformation_in_time_region(transformation_id, world) {
        protocol_slot <= EXECUTION_DEADLINE_SLOT
    } else {
        true
    }
}

fn current_evidence_for_candidate(transformation_id: usize, world: &VisibleWorld) -> Evidence {
    let in_time_region = transformation_in_time_region(transformation_id, world);

    Evidence {
        dependency_id: world.dependency_id,
        account_digest: world.account_digest,
        observed_slot: world.current_protocol_slot,
        valid_until_slot: if in_time_region {
            EXECUTION_DEADLINE_SLOT
        } else {
            u64::MAX
        },
        edge_adjustment: if in_time_region {
            TIME_REGION_EDGE_ADJUSTMENT
        } else {
            0
        },
    }
}

fn cached_time_evidence(world: &VisibleWorld) -> Evidence {
    Evidence {
        dependency_id: world.dependency_id,
        account_digest: world.account_digest,
        observed_slot: CACHED_OBSERVED_SLOT,
        valid_until_slot: EXECUTION_DEADLINE_SLOT,
        edge_adjustment: TIME_REGION_EDGE_ADJUSTMENT,
    }
}

fn current_standard_evidence(world: &VisibleWorld) -> Evidence {
    Evidence {
        dependency_id: world.dependency_id,
        account_digest: world.account_digest,
        observed_slot: world.current_protocol_slot,
        valid_until_slot: u64::MAX,
        edge_adjustment: 0,
    }
}

fn candidate_profit(transformation_id: usize, quantity: usize, evidence: Evidence) -> i64 {
    let quantity = quantity as i64;
    let edge = transformation_edge(transformation_id) + evidence.edge_adjustment;
    let impact = transformation_impact(transformation_id);

    quantity * edge - impact * quantity * quantity - FIXED_COST
}

fn decision_at_protocol_slot(
    transformation_id: usize,
    quantity: usize,
    evidence: Evidence,
    protocol_slot: u64,
    world: &VisibleWorld,
) -> Decision {
    let capability_enabled =
        protocol_capability_enabled(transformation_id, protocol_slot, world);

    if capability_enabled && candidate_profit(transformation_id, quantity, evidence) > 0 {
        Decision::Advance
    } else {
        Decision::Reject
    }
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
        execution_enabled: protocol_capability_enabled(
            transformation_id,
            world.current_protocol_slot,
            world,
        ),
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
    work.fact_accesses += 7;

    let evidence = current_evidence_for_candidate(transformation_id, world);

    decision_at_protocol_slot(
        transformation_id,
        quantity,
        evidence,
        world.current_protocol_slot,
        world,
    )
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

fn family_fully_in_time_region(family: CandidateFamily, world: &VisibleWorld) -> bool {
    family.id_start >= world.time_region_start && family.id_end <= world.time_region_end
}

fn family_crosses_time_boundary(family: CandidateFamily, world: &VisibleWorld) -> bool {
    let crosses_start =
        family.id_start < world.time_region_start && family.id_end > world.time_region_start;

    let crosses_end =
        family.id_start < world.time_region_end && family.id_end > world.time_region_end;

    crosses_start || crosses_end
}

fn byte_only_membrane_allows(evidence: Evidence, world: &VisibleWorld) -> bool {
    evidence.dependency_id == world.dependency_id
        && evidence.account_digest == world.account_digest
}

fn proof_membrane_allows(evidence: Evidence, world: &VisibleWorld, work: &mut WorkCounter) -> bool {
    work.proof_checks += 1;
    work.fact_accesses += 4;

    let dependency_matches = evidence.dependency_id == world.dependency_id;
    let bytes_match = evidence.account_digest == world.account_digest;
    let evidence_was_valid = evidence.observed_slot <= evidence.valid_until_slot;
    let evidence_still_valid = world.current_protocol_slot <= evidence.valid_until_slot;

    let allowed =
        dependency_matches && bytes_match && evidence_was_valid && evidence_still_valid;

    if !allowed {
        work.expired_proofs_blocked += 1;
    }

    allowed
}

fn optimistic_profit_at_quantity(quantity: usize, evidence: Evidence) -> i64 {
    let quantity = quantity as i64;
    let edge = MAX_EDGE + evidence.edge_adjustment;

    quantity * edge - MIN_IMPACT * quantity * quantity - FIXED_COST
}

fn pessimistic_profit_at_quantity(quantity: usize, evidence: Evidence) -> i64 {
    let quantity = quantity as i64;
    let edge = MIN_EDGE + evidence.edge_adjustment;

    quantity * edge - MAX_IMPACT * quantity * quantity - FIXED_COST
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

fn family_pessimistic_profit_lower_bound(
    family: CandidateFamily,
    evidence: Evidence,
    work: &mut WorkCounter,
) -> i64 {
    work.family_evaluations += 1;
    work.economic_evaluations += 1;
    work.bound_evaluations += 1;
    work.fact_accesses += 4;

    if evidence.observed_slot > evidence.valid_until_slot {
        return i64::MIN;
    }

    let first_quantity = family.quantity_start;
    let last_quantity = family.quantity_end - 1;

    let first_profit = pessimistic_profit_at_quantity(first_quantity, evidence);
    let last_profit = pessimistic_profit_at_quantity(last_quantity, evidence);

    first_profit.min(last_profit)
}

fn family_proven_reject(
    family: CandidateFamily,
    evidence: Evidence,
    work: &mut WorkCounter,
) -> bool {
    family_optimistic_profit_upper_bound(family, evidence, work) <= 0
}

fn family_proven_advance(
    family: CandidateFamily,
    evidence: Evidence,
    work: &mut WorkCounter,
) -> bool {
    family_pessimistic_profit_lower_bound(family, evidence, work) > 0
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

fn emit_expired_family_advance(family: CandidateFamily, result: &mut EngineResult) {
    result.work.family_advances += 1;
    result.work.stale_authorized_decisions += family.point_count() as u64;

    for transformation_id in family.id_start..family.id_end {
        for quantity in family.quantity_start..family.quantity_end {
            result.advances.insert(CandidateKey {
                transformation_id,
                quantity,
            });
        }
    }
}

fn resolve_time_sensitive_family(
    family: CandidateFamily,
    world: &VisibleWorld,
    result: &mut EngineResult,
) {
    let evidence = cached_time_evidence(world);
    let expired_advance_proof =
        family_proven_advance(family, evidence, &mut result.work);

    if expired_advance_proof && byte_only_membrane_allows(evidence, world) {
        result.work.byte_only_false_accepts += family.point_count() as u64;
    }

    let allowed = proof_membrane_allows(evidence, world, &mut result.work);

    if expired_advance_proof && !allowed {
        result.work.expired_advance_proofs_blocked += 1;
    }

    if expired_advance_proof && allowed {
        emit_expired_family_advance(family, result);
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

fn resolve_fresh_family(family: CandidateFamily, world: &VisibleWorld, result: &mut EngineResult) {
    let evidence = current_standard_evidence(world);

    if family_proven_reject(family, evidence, &mut result.work)
        && proof_membrane_allows(evidence, world, &mut result.work)
    {
        result.rejects += family.point_count();
        result.work.family_rejections += 1;
        result.work.fresh_proof_authorizations += 1;
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

fn resolve_family(family: CandidateFamily, world: &VisibleWorld, result: &mut EngineResult) {
    if family.is_empty() {
        return;
    }

    if family_crosses_time_boundary(family, world) && family.id_len() > 1 {
        result.work.family_splits += 1;
        let (left, right) = split_family(family);
        resolve_family(left, world, result);
        resolve_family(right, world, result);
        return;
    }

    if family_fully_in_time_region(family, world) {
        resolve_time_sensitive_family(family, world, result);
    } else {
        resolve_fresh_family(family, world, result);
    }
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

fn protocol_time_fixture_is_valid(world: &VisibleWorld) -> bool {
    let cached = cached_time_evidence(world);
    let current = current_evidence_for_candidate(world.time_region_start, world);

    cached.observed_slot <= cached.valid_until_slot
        && world.current_protocol_slot > cached.valid_until_slot
        && cached.account_digest == current.account_digest
        && cached.dependency_id == current.dependency_id
        && cached.edge_adjustment == current.edge_adjustment
}

fn protocol_time_reversal_count(world: &VisibleWorld) -> usize {
    let cached_evidence = cached_time_evidence(world);
    let mut reversals = 0;

    for transformation_id in world.time_region_start..world.time_region_end {
        let current_evidence = current_evidence_for_candidate(transformation_id, world);

        for quantity in world.min_quantity..=world.max_quantity {
            let cached_decision = decision_at_protocol_slot(
                transformation_id,
                quantity,
                cached_evidence,
                cached_evidence.observed_slot,
                world,
            );

            let current_decision = decision_at_protocol_slot(
                transformation_id,
                quantity,
                current_evidence,
                world.current_protocol_slot,
                world,
            );

            if cached_decision == Decision::Advance && current_decision == Decision::Reject {
                reversals += 1;
            }
        }
    }

    reversals
}

fn print_work(label: &str, work: &WorkCounter) {
    println!("{label}");
    println!(
        "  family evaluations:           {}",
        work.family_evaluations
    );
    println!("  family splits:                {}", work.family_splits);
    println!("  family rejections:            {}", work.family_rejections);
    println!("  family advances:              {}", work.family_advances);
    println!("  exact expansions:             {}", work.exact_expansions);
    println!(
        "  economic evaluations:         {}",
        work.economic_evaluations
    );
    println!("  bound evaluations:            {}", work.bound_evaluations);
    println!("  fact accesses:                 {}", work.fact_accesses);
    println!("  proof checks:                  {}", work.proof_checks);
    println!(
        "  expired proofs blocked:        {}",
        work.expired_proofs_blocked
    );
    println!(
        "  expired ADVANCE proofs blocked:{}",
        work.expired_advance_proofs_blocked
    );
    println!(
        "  byte-only false accepts:       {}",
        work.byte_only_false_accepts
    );
    println!(
        "  fresh proof authorizations:    {}",
        work.fresh_proof_authorizations
    );
    println!(
        "  stale-authorized decisions:    {}",
        work.stale_authorized_decisions
    );
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

    let fixture_valid = protocol_time_fixture_is_valid(&world);
    let reversals = protocol_time_reversal_count(&world);
    let baseline_matches_oracle = baseline.advances == oracle;
    let pulse_matches_oracle = pulse.advances == oracle;
    let decision_agreement = baseline.advances == pulse.advances;
    let false_important_prunes = oracle.difference(&pulse.advances).count();

    println!("Fixture: EZ-006 — Protocol Time Without Bytes");
    println!("Total candidate states: {total_candidate_states}");
    println!("Cached observed slot: {CACHED_OBSERVED_SLOT}");
    println!("Execution deadline slot: {EXECUTION_DEADLINE_SLOT}");
    println!("Current protocol slot: {}", world.current_protocol_slot);
    println!("Account digest unchanged: {ACCOUNT_DIGEST:#X}");
    println!("Protocol-time fixture valid: {fixture_valid}");
    println!("Protocol-time reversal count: {reversals}");
    println!("Oracle ADVANCE count: {}", oracle.len());
    println!();

    println!("Correctness");
    println!("  baseline matches oracle: {baseline_matches_oracle}");
    println!("  pulse matches oracle:    {pulse_matches_oracle}");
    println!("  decision agreement:      {decision_agreement}");
    println!("  false important prunes:  {false_important_prunes}");
    println!(
        "  byte-only false accepts:     {}",
        pulse.work.byte_only_false_accepts
    );
    println!(
        "  stale-authorized decisions:  {}",
        pulse.work.stale_authorized_decisions
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
        && reversals > 0
        && pulse.work.byte_only_false_accepts > 0
        && pulse.work.expired_proofs_blocked > 0
        && pulse.work.expired_advance_proofs_blocked > 0
        && pulse.work.stale_authorized_decisions == 0
        && baseline_matches_oracle
        && pulse_matches_oracle
        && decision_agreement
        && false_important_prunes == 0
        && baseline.rejects == pulse.rejects;

    println!();

    if passed {
        println!("EZ-006 CORRECTNESS GATE: PASS");
    } else {
        println!("EZ-006 CORRECTNESS GATE: FAIL");
        std::process::exit(1);
    }
}
