use std::collections::BTreeSet;

const TRANSFORMATION_COUNT: usize = 100;
const MIN_QUANTITY: usize = 1;
const MAX_QUANTITY: usize = 100;

const ALPHA_REGION_START: usize = 20;
const ALPHA_REGION_END: usize = 40;
const STABLE_REGION_START: usize = 60;
const STABLE_REGION_END: usize = 70;

const DEFAULT_BASE_EDGE: i64 = 10;
const ALPHA_BASE_EDGE: i64 = 18;
const STABLE_BASE_EDGE: i64 = 24;

const PHASE_B_ALPHA_ADJUSTMENT: i64 = 3;
const IMPACT: i64 = 1;
const FIXED_COST: i64 = 100;

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
    alpha_adjustment: i64,
}

#[derive(Debug, Clone, Copy)]
struct DecisionCertificate {
    max_positive_alpha_adjustment: i64,
}

#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    key: CandidateKey,
    decision: Decision,
    alpha_dependent: bool,
    certificate: Option<DecisionCertificate>,
}

#[derive(Debug, Default, Clone)]
struct WorkCounter {
    exact_expansions: u64,
    economic_evaluations: u64,
    fact_accesses: u64,
    cache_checks: u64,
    cache_reuses: u64,
    dependency_invalidations: u64,
    certificate_checks: u64,
    certificate_firewalls: u64,
    semantic_invalidations: u64,
}

#[derive(Debug)]
struct RunResult {
    decisions: Vec<Decision>,
    work: WorkCounter,
}

#[derive(Debug)]
struct PulseResult {
    decisions: Vec<Decision>,
    invalidated_keys: BTreeSet<CandidateKey>,
    firewalled_keys: BTreeSet<CandidateKey>,
    work: WorkCounter,
}

fn phase_a_world() -> World {
    World {
        alpha_adjustment: 0,
    }
}

fn phase_b_world() -> World {
    World {
        alpha_adjustment: PHASE_B_ALPHA_ADJUSTMENT,
    }
}

fn total_candidate_states() -> usize {
    TRANSFORMATION_COUNT * (MAX_QUANTITY - MIN_QUANTITY + 1)
}

fn in_region(transformation_id: usize, start: usize, end: usize) -> bool {
    (start..end).contains(&transformation_id)
}

fn is_alpha_dependent(transformation_id: usize) -> bool {
    in_region(transformation_id, ALPHA_REGION_START, ALPHA_REGION_END)
}

fn base_edge(transformation_id: usize) -> i64 {
    if in_region(
        transformation_id,
        STABLE_REGION_START,
        STABLE_REGION_END,
    ) {
        STABLE_BASE_EDGE
    } else if is_alpha_dependent(transformation_id) {
        ALPHA_BASE_EDGE
    } else {
        DEFAULT_BASE_EDGE
    }
}

fn world_adjustment(transformation_id: usize, world: &World) -> i64 {
    if is_alpha_dependent(transformation_id) {
        world.alpha_adjustment
    } else {
        0
    }
}

fn profit(transformation_id: usize, quantity: usize, world: &World) -> i64 {
    let quantity = quantity as i64;
    let edge = base_edge(transformation_id) + world_adjustment(transformation_id, world);

    quantity * edge - IMPACT * quantity * quantity - FIXED_COST
}

fn oracle_decision(transformation_id: usize, quantity: usize, world: &World) -> Decision {
    if profit(transformation_id, quantity, world) > 0 {
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
    work.fact_accesses += 2;

    oracle_decision(transformation_id, quantity, world)
}

fn run_oracle(world: &World) -> Vec<Decision> {
    let mut decisions = Vec::with_capacity(total_candidate_states());

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            decisions.push(oracle_decision(transformation_id, quantity, world));
        }
    }

    decisions
}

fn build_reject_certificate(
    transformation_id: usize,
    quantity: usize,
    world: &World,
    decision: Decision,
) -> Option<DecisionCertificate> {
    if !is_alpha_dependent(transformation_id) || decision != Decision::Reject {
        return None;
    }

    let current_profit = profit(transformation_id, quantity, world);
    assert!(current_profit <= 0);

    let quantity = quantity as i64;
    let max_positive_alpha_adjustment = (-current_profit) / quantity;

    Some(DecisionCertificate {
        max_positive_alpha_adjustment,
    })
}

fn build_phase_a_cache(world: &World) -> (Vec<CacheEntry>, WorkCounter) {
    let mut cache = Vec::with_capacity(total_candidate_states());
    let mut work = WorkCounter::default();

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            let key = CandidateKey {
                transformation_id,
                quantity,
            };
            let decision = evaluate_exact(transformation_id, quantity, world, &mut work);
            let alpha_dependent = is_alpha_dependent(transformation_id);
            let certificate =
                build_reject_certificate(transformation_id, quantity, world, decision);

            cache.push(CacheEntry {
                key,
                decision,
                alpha_dependent,
                certificate,
            });
        }
    }

    (cache, work)
}

fn run_global_recompute(world: &World) -> RunResult {
    let mut decisions = Vec::with_capacity(total_candidate_states());
    let mut work = WorkCounter::default();

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            let decision = evaluate_exact(transformation_id, quantity, world, &mut work);
            decisions.push(decision);
        }
    }

    RunResult { decisions, work }
}

fn run_dependency_only(cache: &[CacheEntry], world: &World) -> RunResult {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        if entry.alpha_dependent {
            work.dependency_invalidations += 1;

            let decision = evaluate_exact(
                entry.key.transformation_id,
                entry.key.quantity,
                world,
                &mut work,
            );
            decisions.push(decision);
        } else {
            work.cache_reuses += 1;
            decisions.push(entry.decision);
        }
    }

    RunResult { decisions, work }
}

fn run_unsafe_semantic_reuse(cache: &[CacheEntry]) -> RunResult {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;
        work.cache_reuses += 1;
        decisions.push(entry.decision);
    }

    RunResult { decisions, work }
}

fn certificate_allows_reuse(
    entry: &CacheEntry,
    current_alpha_adjustment: i64,
    work: &mut WorkCounter,
) -> bool {
    work.certificate_checks += 1;

    match entry.certificate {
        Some(certificate) => {
            current_alpha_adjustment <= certificate.max_positive_alpha_adjustment
        }
        None => false,
    }
}

fn run_pulse_semantic(cache: &[CacheEntry], world: &World) -> PulseResult {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut invalidated_keys = BTreeSet::new();
    let mut firewalled_keys = BTreeSet::new();
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        if !entry.alpha_dependent {
            work.cache_reuses += 1;
            decisions.push(entry.decision);
            continue;
        }

        work.dependency_invalidations += 1;

        if certificate_allows_reuse(entry, world.alpha_adjustment, &mut work) {
            work.certificate_firewalls += 1;
            work.cache_reuses += 1;
            firewalled_keys.insert(entry.key);
            decisions.push(entry.decision);
        } else {
            work.semantic_invalidations += 1;
            invalidated_keys.insert(entry.key);

            let decision = evaluate_exact(
                entry.key.transformation_id,
                entry.key.quantity,
                world,
                &mut work,
            );
            decisions.push(decision);
        }
    }

    PulseResult {
        decisions,
        invalidated_keys,
        firewalled_keys,
        work,
    }
}

fn dependency_cone(cache: &[CacheEntry]) -> BTreeSet<CandidateKey> {
    cache
        .iter()
        .filter(|entry| entry.alpha_dependent)
        .map(|entry| entry.key)
        .collect()
}

fn changed_decision_keys(
    phase_a: &[Decision],
    phase_b: &[Decision],
    cache: &[CacheEntry],
) -> BTreeSet<CandidateKey> {
    assert_eq!(phase_a.len(), phase_b.len());
    assert_eq!(phase_a.len(), cache.len());

    phase_a
        .iter()
        .zip(phase_b)
        .zip(cache)
        .filter_map(|((before, after), entry)| {
            if before != after {
                Some(entry.key)
            } else {
                None
            }
        })
        .collect()
}

fn decision_mismatches(reference: &[Decision], candidate: &[Decision]) -> usize {
    assert_eq!(reference.len(), candidate.len());

    reference
        .iter()
        .zip(candidate)
        .filter(|(expected, actual)| expected != actual)
        .count()
}

fn advance_count(decisions: &[Decision]) -> usize {
    decisions
        .iter()
        .filter(|decision| **decision == Decision::Advance)
        .count()
}

fn print_work(label: &str, work: &WorkCounter) {
    println!("{label}");
    println!("  exact expansions:          {}", work.exact_expansions);
    println!(
        "  economic evaluations:      {}",
        work.economic_evaluations
    );
    println!("  fact accesses:             {}", work.fact_accesses);
    println!("  cache checks:              {}", work.cache_checks);
    println!("  cache reuses:              {}", work.cache_reuses);
    println!(
        "  dependency invalidations:  {}",
        work.dependency_invalidations
    );
    println!("  certificate checks:        {}", work.certificate_checks);
    println!(
        "  certificate firewalls:     {}",
        work.certificate_firewalls
    );
    println!(
        "  semantic invalidations:    {}",
        work.semantic_invalidations
    );
}

fn main() {
    println!("Pulse Field Lab");
    println!("EZ-M0A — Compression Without Loss");
    println!();

    let phase_a = phase_a_world();
    let phase_b = phase_b_world();

    let oracle_a = run_oracle(&phase_a);
    let oracle_b = run_oracle(&phase_b);

    let (phase_a_cache, cache_build_work) = build_phase_a_cache(&phase_a);
    let global_b = run_global_recompute(&phase_b);
    let dependency_b = run_dependency_only(&phase_a_cache, &phase_b);
    let unsafe_b = run_unsafe_semantic_reuse(&phase_a_cache);
    let pulse_b = run_pulse_semantic(&phase_a_cache, &phase_b);

    let cache_decisions: Vec<Decision> =
        phase_a_cache.iter().map(|entry| entry.decision).collect();

    let dependency_keys = dependency_cone(&phase_a_cache);
    let changed_truth = changed_decision_keys(&oracle_a, &oracle_b, &phase_a_cache);

    let expected_firewalled: BTreeSet<CandidateKey> =
        dependency_keys.difference(&changed_truth).copied().collect();

    let cache_matches_phase_a = cache_decisions == oracle_a;
    let global_matches_oracle = global_b.decisions == oracle_b;
    let dependency_matches_oracle = dependency_b.decisions == oracle_b;
    let pulse_matches_oracle = pulse_b.decisions == oracle_b;

    let unsafe_stale_authorized = decision_mismatches(&oracle_b, &unsafe_b.decisions);
    let pulse_stale_authorized = decision_mismatches(&oracle_b, &pulse_b.decisions);

    let missed_semantic_invalidations = changed_truth
        .difference(&pulse_b.invalidated_keys)
        .count();

    let false_semantic_invalidations = pulse_b
        .invalidated_keys
        .difference(&changed_truth)
        .count();

    let missed_firewalls = expected_firewalled
        .difference(&pulse_b.firewalled_keys)
        .count();

    let false_firewalls = pulse_b
        .firewalled_keys
        .difference(&expected_firewalled)
        .count();

    let total_states = total_candidate_states();
    let unaffected_entries = total_states - dependency_keys.len();

    println!("Fixture: EZ-010 — Semantic Invalidation / Decision Firewall");
    println!("Total candidate states: {total_states}");
    println!(
        "Alpha dependency region: {}..{}",
        ALPHA_REGION_START, ALPHA_REGION_END
    );
    println!(
        "Stable ADVANCE region:    {}..{}",
        STABLE_REGION_START, STABLE_REGION_END
    );
    println!(
        "Phase B Alpha adjustment: {}",
        phase_b.alpha_adjustment
    );
    println!();

    println!("Semantic topology");
    println!(
        "  changed dependency cone:       {}",
        dependency_keys.len()
    );
    println!("  unaffected entries:            {unaffected_entries}");
    println!("  decisions that truly changed:  {}", changed_truth.len());
    println!(
        "  provably invariant dependents: {}",
        expected_firewalled.len()
    );
    println!();

    println!("Oracle");
    println!("  Phase A ADVANCE: {}", advance_count(&oracle_a));
    println!("  Phase B ADVANCE: {}", advance_count(&oracle_b));
    println!();

    println!("Unsafe semantic reuse");
    println!(
        "  ADVANCE:                    {}",
        advance_count(&unsafe_b.decisions)
    );
    println!(
        "  stale-authorized decisions: {unsafe_stale_authorized}"
    );
    println!("  exact recomputations:       {}", unsafe_b.work.exact_expansions);
    println!();

    println!("Correctness");
    println!("  Phase A cache matches Oracle:       {cache_matches_phase_a}");
    println!("  global matches Oracle:              {global_matches_oracle}");
    println!(
        "  dependency-only matches Oracle:     {dependency_matches_oracle}"
    );
    println!("  Pulse matches Oracle:               {pulse_matches_oracle}");
    println!("  Pulse stale-authorized:             {pulse_stale_authorized}");
    println!(
        "  missed semantic invalidations:      {missed_semantic_invalidations}"
    );
    println!(
        "  false semantic invalidations:       {false_semantic_invalidations}"
    );
    println!("  missed certificate firewalls:       {missed_firewalls}");
    println!("  false certificate firewalls:        {false_firewalls}");
    println!(
        "  Pulse semantic invalidations:       {}",
        pulse_b.invalidated_keys.len()
    );
    println!(
        "  Pulse certificate firewalls:        {}",
        pulse_b.firewalled_keys.len()
    );
    println!();

    print_work("Phase A cache build work", &cache_build_work);
    println!();
    print_work("Global Phase B recomputation", &global_b.work);
    println!();
    print_work("Dependency-only Phase B invalidation", &dependency_b.work);
    println!();
    print_work("Pulse Phase B semantic invalidation", &pulse_b.work);
    println!();

    let dependency_expansions = dependency_b.work.exact_expansions as f64;
    let pulse_expansions = pulse_b.work.exact_expansions as f64;
    let semantic_reduction =
        100.0 * (dependency_expansions - pulse_expansions) / dependency_expansions;

    let global_expansions = global_b.work.exact_expansions as f64;
    let global_reduction =
        100.0 * (global_expansions - pulse_expansions) / global_expansions;

    println!(
        "Reduction vs dependency-only recomputation: {:.2}%",
        semantic_reduction
    );
    println!(
        "Reduction vs global recomputation:          {:.2}%",
        global_reduction
    );

    let passed = cache_matches_phase_a
        && global_matches_oracle
        && dependency_matches_oracle
        && pulse_matches_oracle
        && unsafe_stale_authorized > 0
        && pulse_stale_authorized == 0
        && missed_semantic_invalidations == 0
        && false_semantic_invalidations == 0
        && missed_firewalls == 0
        && false_firewalls == 0
        && pulse_b.invalidated_keys == changed_truth
        && pulse_b.firewalled_keys == expected_firewalled
        && pulse_b.work.semantic_invalidations == changed_truth.len() as u64
        && pulse_b.work.certificate_firewalls == expected_firewalled.len() as u64
        && pulse_b.work.exact_expansions == changed_truth.len() as u64
        && pulse_b.work.exact_expansions < dependency_b.work.exact_expansions
        && dependency_b.work.exact_expansions < global_b.work.exact_expansions;

    println!();

    if passed {
        println!("EZ-010 CORRECTNESS GATE: PASS");
    } else {
        println!("EZ-010 CORRECTNESS GATE: FAIL");
        std::process::exit(1);
    }
}
