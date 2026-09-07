use std::collections::BTreeSet;

const TRANSFORMATION_COUNT: usize = 100;
const MIN_QUANTITY: usize = 1;
const MAX_QUANTITY: usize = 100;

const ALPHA_REGION_START: usize = 20;
const ALPHA_REGION_END: usize = 26;
const BETA_REGION_START: usize = 23;
const BETA_REGION_END: usize = 30;
const GAMMA_REGION_START: usize = 60;
const GAMMA_REGION_END: usize = 70;

const ALPHA_FACT: u8 = 1;
const BETA_FACT: u8 = 1 << 1;
const GAMMA_FACT: u8 = 1 << 2;

const DEFAULT_BASE_EDGE: i64 = 10;
const DEPENDENCY_BASE_EDGE: i64 = 18;
const STABLE_ADVANCE_BASE_EDGE: i64 = 24;

const PHASE_B_ALPHA_ADJUSTMENT: i64 = 8;
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
    beta_adjustment: i64,
    gamma_adjustment: i64,
}

#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    key: CandidateKey,
    decision: Decision,
    dependencies: u8,
}

#[derive(Debug, Default, Clone)]
struct WorkCounter {
    exact_expansions: u64,
    economic_evaluations: u64,
    fact_accesses: u64,
    cache_checks: u64,
    cache_reuses: u64,
    cache_invalidations: u64,
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
    work: WorkCounter,
}

fn phase_a_world() -> World {
    World {
        alpha_adjustment: 0,
        beta_adjustment: 0,
        gamma_adjustment: 0,
    }
}

fn phase_b_world() -> World {
    World {
        alpha_adjustment: PHASE_B_ALPHA_ADJUSTMENT,
        beta_adjustment: 0,
        gamma_adjustment: 0,
    }
}

fn total_candidate_states() -> usize {
    TRANSFORMATION_COUNT * (MAX_QUANTITY - MIN_QUANTITY + 1)
}

fn in_region(transformation_id: usize, start: usize, end: usize) -> bool {
    (start..end).contains(&transformation_id)
}

fn dependency_mask(transformation_id: usize) -> u8 {
    let mut dependencies = 0;

    if in_region(transformation_id, ALPHA_REGION_START, ALPHA_REGION_END) {
        dependencies |= ALPHA_FACT;
    }

    if in_region(transformation_id, BETA_REGION_START, BETA_REGION_END) {
        dependencies |= BETA_FACT;
    }

    if in_region(transformation_id, GAMMA_REGION_START, GAMMA_REGION_END) {
        dependencies |= GAMMA_FACT;
    }

    dependencies
}

fn base_edge(transformation_id: usize) -> i64 {
    if in_region(transformation_id, GAMMA_REGION_START, GAMMA_REGION_END) {
        STABLE_ADVANCE_BASE_EDGE
    } else if in_region(transformation_id, ALPHA_REGION_START, BETA_REGION_END) {
        DEPENDENCY_BASE_EDGE
    } else {
        DEFAULT_BASE_EDGE
    }
}

fn world_adjustment(transformation_id: usize, world: &World) -> i64 {
    let dependencies = dependency_mask(transformation_id);
    let mut adjustment = 0;

    if dependencies & ALPHA_FACT != 0 {
        adjustment += world.alpha_adjustment;
    }

    if dependencies & BETA_FACT != 0 {
        adjustment += world.beta_adjustment;
    }

    if dependencies & GAMMA_FACT != 0 {
        adjustment += world.gamma_adjustment;
    }

    adjustment
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
    work.fact_accesses += 4;

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

            cache.push(CacheEntry {
                key,
                decision,
                dependencies: dependency_mask(transformation_id),
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

fn run_unsafe_stale_reuse(cache: &[CacheEntry]) -> RunResult {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;
        work.cache_reuses += 1;
        decisions.push(entry.decision);
    }

    RunResult { decisions, work }
}

fn run_pulse_selective(cache: &[CacheEntry], world: &World, changed_facts: u8) -> PulseResult {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut invalidated_keys = BTreeSet::new();
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        if entry.dependencies & changed_facts != 0 {
            work.cache_invalidations += 1;
            invalidated_keys.insert(entry.key);

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

    PulseResult {
        decisions,
        invalidated_keys,
        work,
    }
}

fn expected_affected_keys(cache: &[CacheEntry], changed_facts: u8) -> BTreeSet<CandidateKey> {
    let mut affected = BTreeSet::new();

    for entry in cache {
        if entry.dependencies & changed_facts != 0 {
            affected.insert(entry.key);
        }
    }

    affected
}

fn changed_decision_keys(
    phase_a: &[Decision],
    phase_b: &[Decision],
    cache: &[CacheEntry],
) -> BTreeSet<CandidateKey> {
    assert_eq!(phase_a.len(), phase_b.len());
    assert_eq!(phase_a.len(), cache.len());

    let mut changed = BTreeSet::new();

    for ((before, after), entry) in phase_a.iter().zip(phase_b).zip(cache) {
        if before != after {
            changed.insert(entry.key);
        }
    }

    changed
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

fn count_entries_with_fact(cache: &[CacheEntry], fact: u8) -> usize {
    cache
        .iter()
        .filter(|entry| entry.dependencies & fact != 0)
        .count()
}

fn count_entries_with_both_facts(cache: &[CacheEntry], first: u8, second: u8) -> usize {
    cache
        .iter()
        .filter(|entry| entry.dependencies & first != 0 && entry.dependencies & second != 0)
        .count()
}

fn print_work(label: &str, work: &WorkCounter) {
    println!("{label}");
    println!("  exact expansions:     {}", work.exact_expansions);
    println!("  economic evaluations: {}", work.economic_evaluations);
    println!("  fact accesses:        {}", work.fact_accesses);
    println!("  cache checks:         {}", work.cache_checks);
    println!("  cache reuses:         {}", work.cache_reuses);
    println!("  cache invalidations:  {}", work.cache_invalidations);
}

fn main() {
    println!("Pulse Field Lab");
    println!("EZ-M0A — Compression Without Loss");
    println!();

    let phase_a = phase_a_world();
    let phase_b = phase_b_world();
    let changed_facts = ALPHA_FACT;

    let oracle_a = run_oracle(&phase_a);
    let oracle_b = run_oracle(&phase_b);

    let (phase_a_cache, cache_build_work) = build_phase_a_cache(&phase_a);
    let global_b = run_global_recompute(&phase_b);
    let unsafe_b = run_unsafe_stale_reuse(&phase_a_cache);
    let pulse_b = run_pulse_selective(&phase_a_cache, &phase_b, changed_facts);

    let cache_decisions: Vec<Decision> = phase_a_cache.iter().map(|entry| entry.decision).collect();

    let expected_affected = expected_affected_keys(&phase_a_cache, changed_facts);
    let changed_truth = changed_decision_keys(&oracle_a, &oracle_b, &phase_a_cache);

    let cache_matches_phase_a = cache_decisions == oracle_a;
    let global_matches_oracle = global_b.decisions == oracle_b;
    let pulse_matches_oracle = pulse_b.decisions == oracle_b;

    let stale_authorized_decisions = decision_mismatches(&oracle_b, &unsafe_b.decisions);
    let pulse_stale_authorized_decisions = decision_mismatches(&oracle_b, &pulse_b.decisions);

    let missed_invalidations = expected_affected
        .difference(&pulse_b.invalidated_keys)
        .count();

    let false_invalidations = pulse_b
        .invalidated_keys
        .difference(&expected_affected)
        .count();

    let changed_truth_outside_dependency_cone =
        changed_truth.difference(&expected_affected).count();

    let alpha_entries = count_entries_with_fact(&phase_a_cache, ALPHA_FACT);
    let beta_entries = count_entries_with_fact(&phase_a_cache, BETA_FACT);
    let gamma_entries = count_entries_with_fact(&phase_a_cache, GAMMA_FACT);

    let alpha_beta_overlap = count_entries_with_both_facts(&phase_a_cache, ALPHA_FACT, BETA_FACT);

    let total_states = total_candidate_states();
    let unaffected_entries = total_states - expected_affected.len();

    println!("Fixture: EZ-009 — Revocable Knowledge / Selective Invalidation");
    println!("Total candidate states: {total_states}");
    println!(
        "Alpha dependency region: {}..{}",
        ALPHA_REGION_START, ALPHA_REGION_END
    );
    println!(
        "Beta dependency region:  {}..{}",
        BETA_REGION_START, BETA_REGION_END
    );
    println!(
        "Gamma dependency region: {}..{}",
        GAMMA_REGION_START, GAMMA_REGION_END
    );
    println!("Changed fact mask: {changed_facts}");
    println!();

    println!("Dependency topology");
    println!("  Alpha-dependent entries:       {alpha_entries}");
    println!("  Beta-dependent entries:        {beta_entries}");
    println!("  Gamma-dependent entries:       {gamma_entries}");
    println!("  Alpha/Beta overlap entries:    {alpha_beta_overlap}");
    println!(
        "  Expected affected cone:        {}",
        expected_affected.len()
    );
    println!("  Expected unaffected cache:     {unaffected_entries}");
    println!("  Decisions that truly changed:  {}", changed_truth.len());
    println!("  Truth changes outside cone:    {changed_truth_outside_dependency_cone}");
    println!();

    println!("Oracle");
    println!("  Phase A ADVANCE: {}", advance_count(&oracle_a));
    println!("  Phase B ADVANCE: {}", advance_count(&oracle_b));
    println!();

    println!("Unsafe stale-cache reuse");
    println!(
        "  ADVANCE:                   {}",
        advance_count(&unsafe_b.decisions)
    );
    println!("  stale-authorized decisions: {stale_authorized_decisions}");
    println!(
        "  exact recomputations:       {}",
        unsafe_b.work.exact_expansions
    );
    println!(
        "  stale cache reuses:         {}",
        unsafe_b.work.cache_reuses
    );
    println!();

    println!("Correctness");
    println!("  Phase A cache matches Oracle: {cache_matches_phase_a}");
    println!("  global matches Oracle:        {global_matches_oracle}");
    println!("  Pulse matches Oracle:         {pulse_matches_oracle}");
    println!("  Pulse stale-authorized:       {pulse_stale_authorized_decisions}");
    println!("  missed invalidations:         {missed_invalidations}");
    println!("  false invalidations:          {false_invalidations}");
    println!(
        "  invalidated entries:          {}",
        pulse_b.invalidated_keys.len()
    );
    println!(
        "  unaffected entries reused:    {}",
        pulse_b.work.cache_reuses
    );
    println!();

    print_work("Phase A cache build work", &cache_build_work);
    println!();
    print_work("Global Phase B recomputation", &global_b.work);
    println!();
    print_work("Pulse Phase B selective invalidation", &pulse_b.work);
    println!();

    let global_expansions = global_b.work.exact_expansions as f64;
    let pulse_expansions = pulse_b.work.exact_expansions as f64;
    let expansion_reduction = 100.0 * (global_expansions - pulse_expansions) / global_expansions;

    println!(
        "Selective recomputation reduction: {:.2}%",
        expansion_reduction
    );

    let passed = cache_matches_phase_a
        && global_matches_oracle
        && pulse_matches_oracle
        && stale_authorized_decisions > 0
        && pulse_stale_authorized_decisions == 0
        && missed_invalidations == 0
        && false_invalidations == 0
        && changed_truth_outside_dependency_cone == 0
        && pulse_b.invalidated_keys == expected_affected
        && pulse_b.work.cache_reuses == unaffected_entries as u64
        && pulse_b.work.cache_invalidations == expected_affected.len() as u64
        && pulse_b.work.exact_expansions == expected_affected.len() as u64
        && pulse_b.work.exact_expansions < global_b.work.exact_expansions;

    println!();

    if passed {
        println!("EZ-009 CORRECTNESS GATE: PASS");
    } else {
        println!("EZ-009 CORRECTNESS GATE: FAIL");
        std::process::exit(1);
    }
}
