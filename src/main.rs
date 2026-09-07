const TRANSFORMATION_COUNT: usize = 100;
const MIN_QUANTITY: usize = 1;
const MAX_QUANTITY: usize = 100;

const ALPHA_REGION_START: usize = 20;
const ALPHA_REGION_END: usize = 40;
const BETA_REGION_START: usize = 30;
const BETA_REGION_END: usize = 50;

const DEFAULT_BASE_EDGE: i64 = 10;
const DEPENDENCY_BASE_EDGE: i64 = 18;
const FIXED_COST: i64 = 100;
const IMPACT: i64 = 1;

const GENERATION_ONE: u64 = 1;
const GENERATION_TWO: u64 = 2;
const GENERATION_THREE: u64 = 3;

const ALPHA_SHIFT: i64 = 4;
const BETA_SHIFT: i64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Advance,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FragmentKind {
    Alpha,
    Beta,
}

#[derive(Debug, Clone, Copy)]
struct World {
    generation: u64,
    alpha_adjustment: i64,
    beta_adjustment: i64,
}

#[derive(Debug, Clone, Copy)]
struct ProofFragment {
    kind: FragmentKind,
    generation: u64,
    adjustment: i64,
    canonical: bool,
}

#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    transformation_id: usize,
    quantity: usize,
    decision: Decision,
}

#[derive(Debug, Default, Clone)]
struct WorkCounter {
    exact_expansions: u64,
    economic_evaluations: u64,
    fact_accesses: u64,
    cache_checks: u64,
    cache_reuses: u64,
    dependency_synchronizations: u64,
    coherence_checks: u64,
    proof_compositions: u64,
    incompatible_compositions_refused: u64,
    coherence_reactivations: u64,
}

#[derive(Debug)]
struct RunResult {
    decisions: Vec<Decision>,
    work: WorkCounter,
}

fn generation_one_world() -> World {
    World {
        generation: GENERATION_ONE,
        alpha_adjustment: 0,
        beta_adjustment: 0,
    }
}

fn generation_two_world() -> World {
    World {
        generation: GENERATION_TWO,
        alpha_adjustment: ALPHA_SHIFT,
        beta_adjustment: 0,
    }
}

fn generation_three_world() -> World {
    World {
        generation: GENERATION_THREE,
        alpha_adjustment: 0,
        beta_adjustment: BETA_SHIFT,
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

fn is_beta_dependent(transformation_id: usize) -> bool {
    in_region(transformation_id, BETA_REGION_START, BETA_REGION_END)
}

fn is_dependency_region(transformation_id: usize) -> bool {
    is_alpha_dependent(transformation_id) || is_beta_dependent(transformation_id)
}

fn is_overlap_region(transformation_id: usize) -> bool {
    is_alpha_dependent(transformation_id) && is_beta_dependent(transformation_id)
}

fn base_edge(transformation_id: usize) -> i64 {
    if is_dependency_region(transformation_id) {
        DEPENDENCY_BASE_EDGE
    } else {
        DEFAULT_BASE_EDGE
    }
}

fn world_adjustment(transformation_id: usize, world: &World) -> i64 {
    let mut adjustment = 0;

    if is_alpha_dependent(transformation_id) {
        adjustment += world.alpha_adjustment;
    }

    if is_beta_dependent(transformation_id) {
        adjustment += world.beta_adjustment;
    }

    adjustment
}

fn profit_from_adjustment(transformation_id: usize, quantity: usize, adjustment: i64) -> i64 {
    let quantity = quantity as i64;
    let edge = base_edge(transformation_id) + adjustment;

    quantity * edge - IMPACT * quantity * quantity - FIXED_COST
}

fn decision_from_adjustment(
    transformation_id: usize,
    quantity: usize,
    adjustment: i64,
) -> Decision {
    if profit_from_adjustment(transformation_id, quantity, adjustment) > 0 {
        Decision::Advance
    } else {
        Decision::Reject
    }
}

fn oracle_decision(transformation_id: usize, quantity: usize, world: &World) -> Decision {
    decision_from_adjustment(
        transformation_id,
        quantity,
        world_adjustment(transformation_id, world),
    )
}

fn evaluate_exact(
    transformation_id: usize,
    quantity: usize,
    world: &World,
    work: &mut WorkCounter,
) -> Decision {
    work.exact_expansions += 1;
    work.economic_evaluations += 1;
    work.fact_accesses += 3;

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

fn build_current_cache(world: &World) -> (Vec<CacheEntry>, WorkCounter) {
    let mut cache = Vec::with_capacity(total_candidate_states());
    let mut work = WorkCounter::default();

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            let decision = evaluate_exact(transformation_id, quantity, world, &mut work);

            cache.push(CacheEntry {
                transformation_id,
                quantity,
                decision,
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
            decisions.push(evaluate_exact(
                transformation_id,
                quantity,
                world,
                &mut work,
            ));
        }
    }

    RunResult { decisions, work }
}

fn run_dependency_only(cache: &[CacheEntry], world: &World) -> RunResult {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        if is_dependency_region(entry.transformation_id) {
            work.dependency_synchronizations += 1;
            decisions.push(evaluate_exact(
                entry.transformation_id,
                entry.quantity,
                world,
                &mut work,
            ));
        } else {
            work.cache_reuses += 1;
            decisions.push(entry.decision);
        }
    }

    RunResult { decisions, work }
}

fn fragments_individually_canonical(
    alpha_fragment: ProofFragment,
    beta_fragment: ProofFragment,
) -> bool {
    alpha_fragment.canonical
        && beta_fragment.canonical
        && alpha_fragment.kind == FragmentKind::Alpha
        && beta_fragment.kind == FragmentKind::Beta
}

fn fragments_jointly_coherent(alpha_fragment: ProofFragment, beta_fragment: ProofFragment) -> bool {
    fragments_individually_canonical(alpha_fragment, beta_fragment)
        && alpha_fragment.generation == beta_fragment.generation
}

fn run_unsafe_frankenstein(
    cache: &[CacheEntry],
    alpha_fragment: ProofFragment,
    beta_fragment: ProofFragment,
) -> RunResult {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        if is_overlap_region(entry.transformation_id) {
            work.proof_compositions += 1;
            let fabricated_adjustment = alpha_fragment.adjustment + beta_fragment.adjustment;
            decisions.push(decision_from_adjustment(
                entry.transformation_id,
                entry.quantity,
                fabricated_adjustment,
            ));
        } else {
            work.cache_reuses += 1;
            decisions.push(entry.decision);
        }
    }

    RunResult { decisions, work }
}

fn run_pulse(
    cache: &[CacheEntry],
    current_world: &World,
    alpha_fragment: ProofFragment,
    beta_fragment: ProofFragment,
) -> RunResult {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        if !is_overlap_region(entry.transformation_id) {
            work.cache_reuses += 1;
            decisions.push(entry.decision);
            continue;
        }

        work.coherence_checks += 1;

        if fragments_jointly_coherent(alpha_fragment, beta_fragment) {
            work.proof_compositions += 1;
            let composed_adjustment = alpha_fragment.adjustment + beta_fragment.adjustment;
            decisions.push(decision_from_adjustment(
                entry.transformation_id,
                entry.quantity,
                composed_adjustment,
            ));
        } else {
            work.incompatible_compositions_refused += 1;
            work.coherence_reactivations += 1;
            decisions.push(evaluate_exact(
                entry.transformation_id,
                entry.quantity,
                current_world,
                &mut work,
            ));
        }
    }

    RunResult { decisions, work }
}

fn advance_count(decisions: &[Decision]) -> usize {
    decisions
        .iter()
        .filter(|decision| **decision == Decision::Advance)
        .count()
}

fn overlap_advance_count(decisions: &[Decision]) -> usize {
    let mut count = 0;
    let mut index = 0;

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for _quantity in MIN_QUANTITY..=MAX_QUANTITY {
            if is_overlap_region(transformation_id)
                && decisions[index] == Decision::Advance
            {
                count += 1;
            }

            index += 1;
        }
    }

    count
}

fn mismatch_counts(reference: &[Decision], candidate: &[Decision]) -> (usize, usize) {
    assert_eq!(reference.len(), candidate.len());

    let mut false_authorizations = 0;
    let mut false_rejects = 0;

    for (expected, actual) in reference.iter().zip(candidate) {
        match (*expected, *actual) {
            (Decision::Reject, Decision::Advance) => false_authorizations += 1,
            (Decision::Advance, Decision::Reject) => false_rejects += 1,
            _ => {}
        }
    }

    (false_authorizations, false_rejects)
}

fn mismatches_outside_overlap(reference: &[Decision], candidate: &[Decision]) -> usize {
    assert_eq!(reference.len(), candidate.len());

    let mut mismatches = 0;
    let mut index = 0;

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for _quantity in MIN_QUANTITY..=MAX_QUANTITY {
            if !is_overlap_region(transformation_id)
                && reference[index] != candidate[index]
            {
                mismatches += 1;
            }

            index += 1;
        }
    }

    mismatches
}

fn dependency_union_count(cache: &[CacheEntry]) -> usize {
    cache
        .iter()
        .filter(|entry| is_dependency_region(entry.transformation_id))
        .count()
}

fn overlap_count(cache: &[CacheEntry]) -> usize {
    cache
        .iter()
        .filter(|entry| is_overlap_region(entry.transformation_id))
        .count()
}

fn print_work(label: &str, work: &WorkCounter) {
    println!("{label}");
    println!("  exact expansions:                    {}", work.exact_expansions);
    println!(
        "  economic evaluations:                {}",
        work.economic_evaluations
    );
    println!("  fact accesses:                       {}", work.fact_accesses);
    println!("  cache checks:                        {}", work.cache_checks);
    println!("  cache reuses:                        {}", work.cache_reuses);
    println!(
        "  dependency synchronizations:         {}",
        work.dependency_synchronizations
    );
    println!("  coherence checks:                    {}", work.coherence_checks);
    println!("  proof compositions:                  {}", work.proof_compositions);
    println!(
        "  incompatible compositions refused:   {}",
        work.incompatible_compositions_refused
    );
    println!(
        "  coherence reactivations:             {}",
        work.coherence_reactivations
    );
}

fn main() {
    println!("Pulse Field Lab");
    println!("EZ-012 — Frankenstein Proof / Decision-Scoped Coherence");
    println!();

    let generation_one = generation_one_world();
    let generation_two = generation_two_world();
    let current_world = generation_three_world();

    let alpha_fragment = ProofFragment {
        kind: FragmentKind::Alpha,
        generation: generation_two.generation,
        adjustment: generation_two.alpha_adjustment,
        canonical: true,
    };

    let beta_fragment = ProofFragment {
        kind: FragmentKind::Beta,
        generation: current_world.generation,
        adjustment: current_world.beta_adjustment,
        canonical: true,
    };

    let generation_one_oracle = run_oracle(&generation_one);
    let generation_two_oracle = run_oracle(&generation_two);
    let current_oracle = run_oracle(&current_world);

    let (current_cache, cache_build_work) = build_current_cache(&current_world);
    let cache_decisions: Vec<Decision> =
        current_cache.iter().map(|entry| entry.decision).collect();

    let global = run_global_recompute(&current_world);
    let dependency_only = run_dependency_only(&current_cache, &current_world);
    let unsafe_frankenstein =
        run_unsafe_frankenstein(&current_cache, alpha_fragment, beta_fragment);
    let pulse = run_pulse(
        &current_cache,
        &current_world,
        alpha_fragment,
        beta_fragment,
    );

    let individually_canonical =
        fragments_individually_canonical(alpha_fragment, beta_fragment);
    let jointly_coherent = fragments_jointly_coherent(alpha_fragment, beta_fragment);

    let authoritative_worlds = [generation_one, generation_two, current_world];
    let fabricated_joint_exists = authoritative_worlds.iter().any(|world| {
        world.alpha_adjustment == alpha_fragment.adjustment
            && world.beta_adjustment == beta_fragment.adjustment
    });

    let dependency_states = dependency_union_count(&current_cache);
    let overlap_states = overlap_count(&current_cache);
    let reusable_non_joint_states = total_candidate_states() - overlap_states;

    let current_oracle_advances = advance_count(&current_oracle);
    let current_overlap_advances = overlap_advance_count(&current_oracle);
    let frankenstein_overlap_advances =
        overlap_advance_count(&unsafe_frankenstein.decisions);

    let (unsafe_false_authorizations, unsafe_false_rejects) =
        mismatch_counts(&current_oracle, &unsafe_frankenstein.decisions);
    let (pulse_false_authorizations, pulse_false_rejects) =
        mismatch_counts(&current_oracle, &pulse.decisions);

    let unsafe_non_joint_mismatches =
        mismatches_outside_overlap(&current_oracle, &unsafe_frankenstein.decisions);
    let pulse_non_joint_mismatches =
        mismatches_outside_overlap(&current_oracle, &pulse.decisions);

    let cache_matches_oracle = cache_decisions == current_oracle;
    let global_matches_oracle = global.decisions == current_oracle;
    let dependency_matches_oracle = dependency_only.decisions == current_oracle;
    let pulse_matches_oracle = pulse.decisions == current_oracle;

    let reduction_vs_dependency = 100.0
        * (1.0
            - pulse.work.exact_expansions as f64
                / dependency_only.work.exact_expansions as f64);
    let reduction_vs_global = 100.0
        * (1.0 - pulse.work.exact_expansions as f64 / global.work.exact_expansions as f64);

    println!("Fixture");
    println!("  total candidate states:              {}", total_candidate_states());
    println!("  dependency union states:             {dependency_states}");
    println!("  joint overlap states:                {overlap_states}");
    println!(
        "  reusable non-joint states:           {}",
        reusable_non_joint_states
    );
    println!(
        "  Generation 1 Alpha/Beta:             {}/{}",
        generation_one.alpha_adjustment, generation_one.beta_adjustment
    );
    println!(
        "  Generation 2 Alpha/Beta:             {}/{}",
        generation_two.alpha_adjustment, generation_two.beta_adjustment
    );
    println!(
        "  Generation 3 Alpha/Beta:             {}/{}",
        current_world.alpha_adjustment, current_world.beta_adjustment
    );
    println!();

    println!("Proof fragments");
    println!("  individually canonical:              {individually_canonical}");
    println!("  jointly coherent:                    {jointly_coherent}");
    println!(
        "  fabricated +4/+4 world exists:       {}",
        fabricated_joint_exists
    );
    println!();

    println!("Ground truth");
    println!(
        "  Generation 1 Oracle ADVANCE:         {}",
        advance_count(&generation_one_oracle)
    );
    println!(
        "  Generation 2 Oracle ADVANCE:         {}",
        advance_count(&generation_two_oracle)
    );
    println!("  Generation 3 Oracle ADVANCE:         {current_oracle_advances}");
    println!("  current overlap ADVANCE:             {current_overlap_advances}");
    println!(
        "  Frankenstein overlap ADVANCE:        {}",
        frankenstein_overlap_advances
    );
    println!();

    println!("Unsafe Frankenstein composer");
    println!("  false authorizations:                {unsafe_false_authorizations}");
    println!("  false rejects:                       {unsafe_false_rejects}");
    println!("  non-joint mismatches:                {unsafe_non_joint_mismatches}");
    println!();

    println!("Correctness");
    println!("  current cache matches Oracle:        {cache_matches_oracle}");
    println!("  global matches Oracle:               {global_matches_oracle}");
    println!("  dependency-only matches Oracle:      {dependency_matches_oracle}");
    println!("  Pulse matches Oracle:                {pulse_matches_oracle}");
    println!("  Pulse false authorizations:          {pulse_false_authorizations}");
    println!("  Pulse false rejects:                 {pulse_false_rejects}");
    println!("  Pulse non-joint mismatches:          {pulse_non_joint_mismatches}");
    println!();

    print_work("Current cache build", &cache_build_work);
    println!();
    print_work("Global recompute", &global.work);
    println!();
    print_work("Dependency-only synchronization", &dependency_only.work);
    println!();
    print_work("Unsafe Frankenstein composer", &unsafe_frankenstein.work);
    println!();
    print_work("Pulse decision-scoped coherence", &pulse.work);
    println!();

    println!(
        "Pulse reduction vs dependency-only:    {:.2}%",
        reduction_vs_dependency
    );
    println!("Pulse reduction vs global:             {:.2}%", reduction_vs_global);
    println!();

    assert_eq!(total_candidate_states(), 10_000);
    assert_eq!(dependency_states, 3_000);
    assert_eq!(overlap_states, 1_000);
    assert_eq!(reusable_non_joint_states, 9_000);

    assert!(individually_canonical);
    assert!(!jointly_coherent);
    assert!(!fabricated_joint_exists);

    assert_eq!(current_oracle_advances, 180);
    assert_eq!(current_overlap_advances, 90);
    assert_eq!(frankenstein_overlap_advances, 170);

    assert!(cache_matches_oracle);
    assert!(global_matches_oracle);
    assert!(dependency_matches_oracle);

    assert_eq!(unsafe_false_authorizations, 80);
    assert_eq!(unsafe_false_rejects, 0);
    assert_eq!(unsafe_non_joint_mismatches, 0);

    assert!(pulse_matches_oracle);
    assert_eq!(pulse_false_authorizations, 0);
    assert_eq!(pulse_false_rejects, 0);
    assert_eq!(pulse_non_joint_mismatches, 0);

    assert_eq!(global.work.exact_expansions, 10_000);
    assert_eq!(dependency_only.work.exact_expansions, 3_000);
    assert_eq!(pulse.work.coherence_checks, 1_000);
    assert_eq!(pulse.work.incompatible_compositions_refused, 1_000);
    assert_eq!(pulse.work.coherence_reactivations, 1_000);
    assert_eq!(pulse.work.exact_expansions, 1_000);
    assert_eq!(pulse.work.cache_reuses, 9_000);
    assert_eq!(pulse.work.proof_compositions, 0);

    assert!((reduction_vs_dependency - 66.666_666).abs() < 0.01);
    assert!((reduction_vs_global - 90.0).abs() < 0.01);

    println!("EZ-012 CORRECTNESS GATE: PASS");
    println!(
        "Invariant: INDIVIDUAL CANONICALITY DOES NOT IMPLY JOINT COHERENCE."
    );
    println!("DO NOT SYNCHRONIZE THE UNIVERSE; SYNCHRONIZE THE PROOF.");
}
