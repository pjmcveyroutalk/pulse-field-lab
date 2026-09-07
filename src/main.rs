const TRANSFORMATION_COUNT: usize = 100;
const MIN_QUANTITY: usize = 1;
const MAX_QUANTITY: usize = 100;

const FAST_REGION_START: usize = 20;
const FAST_REGION_END: usize = 30;
const SLOW_REGION_START: usize = 40;
const SLOW_REGION_END: usize = 60;

const OBSERVED_SLOT: u64 = 100;
const FAST_VALID_UNTIL_SLOT: u64 = 120;
const SLOW_VALID_UNTIL_SLOT: u64 = 180;
const GLOBAL_TTL_20_VALID_UNTIL_SLOT: u64 = 120;
const GLOBAL_TTL_50_VALID_UNTIL_SLOT: u64 = 150;
const CHECKPOINTS: [u64; 6] = [119, 120, 121, 179, 180, 181];

const UNAFFECTED_EDGE: i64 = 10;
const ACTIVE_EDGE: i64 = 102;
const INACTIVE_EDGE: i64 = 0;
const FIXED_COST: i64 = 100;
const IMPACT: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Decision {
    Advance,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemporalKind {
    Down,
    Up,
    Unaffected,
}

#[derive(Debug, Clone, Copy)]
struct CacheEntry {
    transformation_id: usize,
    quantity: usize,
    decision: Decision,
    digest: u64,
}

#[derive(Debug, Clone, Copy)]
struct PulseCacheEntry {
    public: CacheEntry,
    valid_until_slot: Option<u64>,
}

#[derive(Debug, Default, Clone, Copy)]
struct WorkCounter {
    exact_recomputations: u64,
    economic_evaluations: u64,
    cache_checks: u64,
    cache_reuses: u64,
    temporal_checks: u64,
}

#[derive(Debug, Default, Clone, Copy)]
struct ErrorCounter {
    false_authorizations: u64,
    false_rejects: u64,
}

#[derive(Debug, Default, Clone, Copy)]
struct AggregateResult {
    work: WorkCounter,
    errors: ErrorCounter,
}

fn total_candidate_states() -> usize {
    TRANSFORMATION_COUNT * (MAX_QUANTITY - MIN_QUANTITY + 1)
}

fn in_region(transformation_id: usize, start: usize, end: usize) -> bool {
    (start..end).contains(&transformation_id)
}

fn is_fast_region(transformation_id: usize) -> bool {
    in_region(transformation_id, FAST_REGION_START, FAST_REGION_END)
}

fn is_slow_region(transformation_id: usize) -> bool {
    in_region(transformation_id, SLOW_REGION_START, SLOW_REGION_END)
}

fn is_time_sensitive(transformation_id: usize) -> bool {
    is_fast_region(transformation_id) || is_slow_region(transformation_id)
}

fn temporal_kind(transformation_id: usize) -> TemporalKind {
    if is_fast_region(transformation_id) {
        if transformation_id < FAST_REGION_START + (FAST_REGION_END - FAST_REGION_START) / 2 {
            TemporalKind::Down
        } else {
            TemporalKind::Up
        }
    } else if is_slow_region(transformation_id) {
        if transformation_id < SLOW_REGION_START + (SLOW_REGION_END - SLOW_REGION_START) / 2 {
            TemporalKind::Down
        } else {
            TemporalKind::Up
        }
    } else {
        TemporalKind::Unaffected
    }
}

fn valid_until_slot(transformation_id: usize) -> Option<u64> {
    if is_fast_region(transformation_id) {
        Some(FAST_VALID_UNTIL_SLOT)
    } else if is_slow_region(transformation_id) {
        Some(SLOW_VALID_UNTIL_SLOT)
    } else {
        None
    }
}

fn state_digest(transformation_id: usize, quantity: usize) -> u64 {
    let transformation = transformation_id as u64 + 1;
    let quantity = quantity as u64;
    transformation
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .rotate_left(17)
        ^ quantity.wrapping_mul(0xD6E8_FEB8_6659_FD93)
}

fn edge_for_state(transformation_id: usize, current_protocol_slot: u64) -> i64 {
    match temporal_kind(transformation_id) {
        TemporalKind::Unaffected => UNAFFECTED_EDGE,
        TemporalKind::Down => {
            let valid_until = valid_until_slot(transformation_id)
                .expect("time-sensitive Down state must have a validity bound");
            if current_protocol_slot <= valid_until {
                ACTIVE_EDGE
            } else {
                INACTIVE_EDGE
            }
        }
        TemporalKind::Up => {
            let valid_until = valid_until_slot(transformation_id)
                .expect("time-sensitive Up state must have a validity bound");
            if current_protocol_slot <= valid_until {
                INACTIVE_EDGE
            } else {
                ACTIVE_EDGE
            }
        }
    }
}

fn profit(transformation_id: usize, quantity: usize, current_protocol_slot: u64) -> i64 {
    let quantity = quantity as i64;
    let edge = edge_for_state(transformation_id, current_protocol_slot);
    quantity * edge - IMPACT * quantity * quantity - FIXED_COST
}

fn oracle_decision(
    transformation_id: usize,
    quantity: usize,
    current_protocol_slot: u64,
) -> Decision {
    if profit(transformation_id, quantity, current_protocol_slot) > 0 {
        Decision::Advance
    } else {
        Decision::Reject
    }
}

fn evaluate_exact(
    transformation_id: usize,
    quantity: usize,
    current_protocol_slot: u64,
    work: &mut WorkCounter,
) -> Decision {
    work.exact_recomputations += 1;
    work.economic_evaluations += 1;
    oracle_decision(transformation_id, quantity, current_protocol_slot)
}

fn run_oracle(current_protocol_slot: u64) -> Vec<Decision> {
    let mut decisions = Vec::with_capacity(total_candidate_states());

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            decisions.push(oracle_decision(
                transformation_id,
                quantity,
                current_protocol_slot,
            ));
        }
    }

    decisions
}

fn build_slot_100_cache() -> (Vec<CacheEntry>, Vec<PulseCacheEntry>) {
    let mut public_cache = Vec::with_capacity(total_candidate_states());
    let mut pulse_cache = Vec::with_capacity(total_candidate_states());

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            let public = CacheEntry {
                transformation_id,
                quantity,
                decision: oracle_decision(transformation_id, quantity, OBSERVED_SLOT),
                digest: state_digest(transformation_id, quantity),
            };

            public_cache.push(public);
            pulse_cache.push(PulseCacheEntry {
                public,
                valid_until_slot: valid_until_slot(transformation_id),
            });
        }
    }

    (public_cache, pulse_cache)
}

fn assert_cache_integrity(cache: &[CacheEntry]) {
    for entry in cache {
        assert_eq!(
            entry.digest,
            state_digest(entry.transformation_id, entry.quantity)
        );
    }
}

fn mismatch_counts(reference: &[Decision], candidate: &[Decision]) -> ErrorCounter {
    assert_eq!(reference.len(), candidate.len());

    let mut errors = ErrorCounter::default();

    for (expected, actual) in reference.iter().zip(candidate) {
        match (*expected, *actual) {
            (Decision::Reject, Decision::Advance) => errors.false_authorizations += 1,
            (Decision::Advance, Decision::Reject) => errors.false_rejects += 1,
            _ => {}
        }
    }

    errors
}

fn add_work(total: &mut WorkCounter, current: WorkCounter) {
    total.exact_recomputations += current.exact_recomputations;
    total.economic_evaluations += current.economic_evaluations;
    total.cache_checks += current.cache_checks;
    total.cache_reuses += current.cache_reuses;
    total.temporal_checks += current.temporal_checks;
}

fn add_errors(total: &mut ErrorCounter, current: ErrorCounter) {
    total.false_authorizations += current.false_authorizations;
    total.false_rejects += current.false_rejects;
}

fn advance_count(decisions: &[Decision]) -> usize {
    decisions
        .iter()
        .filter(|decision| **decision == Decision::Advance)
        .count()
}

fn run_global_recompute(current_protocol_slot: u64) -> (Vec<Decision>, WorkCounter) {
    let mut decisions = Vec::with_capacity(total_candidate_states());
    let mut work = WorkCounter::default();

    for transformation_id in 0..TRANSFORMATION_COUNT {
        for quantity in MIN_QUANTITY..=MAX_QUANTITY {
            decisions.push(evaluate_exact(
                transformation_id,
                quantity,
                current_protocol_slot,
                &mut work,
            ));
        }
    }

    (decisions, work)
}

fn run_byte_only(cache: &[CacheEntry]) -> (Vec<Decision>, WorkCounter) {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;
        work.cache_reuses += 1;
        decisions.push(entry.decision);
    }

    (decisions, work)
}

fn run_global_ttl(
    cache: &[CacheEntry],
    current_protocol_slot: u64,
    global_valid_until_slot: u64,
) -> (Vec<Decision>, WorkCounter) {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        if is_time_sensitive(entry.transformation_id)
            && current_protocol_slot > global_valid_until_slot
        {
            decisions.push(evaluate_exact(
                entry.transformation_id,
                entry.quantity,
                current_protocol_slot,
                &mut work,
            ));
        } else {
            work.cache_reuses += 1;
            decisions.push(entry.decision);
        }
    }

    (decisions, work)
}

fn run_cached_advance_only(
    cache: &[CacheEntry],
    current_protocol_slot: u64,
) -> (Vec<Decision>, WorkCounter) {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        if is_time_sensitive(entry.transformation_id)
            && current_protocol_slot > GLOBAL_TTL_20_VALID_UNTIL_SLOT
            && entry.decision == Decision::Advance
        {
            decisions.push(evaluate_exact(
                entry.transformation_id,
                entry.quantity,
                current_protocol_slot,
                &mut work,
            ));
        } else {
            work.cache_reuses += 1;
            decisions.push(entry.decision);
        }
    }

    (decisions, work)
}

fn run_cached_reject_only(
    cache: &[CacheEntry],
    current_protocol_slot: u64,
) -> (Vec<Decision>, WorkCounter) {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        if is_time_sensitive(entry.transformation_id)
            && current_protocol_slot > GLOBAL_TTL_20_VALID_UNTIL_SLOT
            && entry.decision == Decision::Reject
        {
            decisions.push(evaluate_exact(
                entry.transformation_id,
                entry.quantity,
                current_protocol_slot,
                &mut work,
            ));
        } else {
            work.cache_reuses += 1;
            decisions.push(entry.decision);
        }
    }

    (decisions, work)
}

fn run_pulse(
    cache: &[PulseCacheEntry],
    current_protocol_slot: u64,
) -> (Vec<Decision>, WorkCounter) {
    let mut decisions = Vec::with_capacity(cache.len());
    let mut work = WorkCounter::default();

    for entry in cache {
        work.cache_checks += 1;

        match entry.valid_until_slot {
            Some(valid_until) => {
                work.temporal_checks += 1;

                if current_protocol_slot > valid_until {
                    decisions.push(evaluate_exact(
                        entry.public.transformation_id,
                        entry.public.quantity,
                        current_protocol_slot,
                        &mut work,
                    ));
                } else {
                    work.cache_reuses += 1;
                    decisions.push(entry.public.decision);
                }
            }
            None => {
                work.cache_reuses += 1;
                decisions.push(entry.public.decision);
            }
        }
    }

    (decisions, work)
}

fn print_policy_result(label: &str, result: &AggregateResult) {
    println!("{label}");
    println!(
        "  false authorizations:                {}",
        result.errors.false_authorizations
    );
    println!(
        "  false rejects:                       {}",
        result.errors.false_rejects
    );
    println!(
        "  exact recomputations:                {}",
        result.work.exact_recomputations
    );
    println!(
        "  economic evaluations:                {}",
        result.work.economic_evaluations
    );
    println!(
        "  cache checks:                        {}",
        result.work.cache_checks
    );
    println!(
        "  cache reuses:                        {}",
        result.work.cache_reuses
    );
    println!(
        "  temporal checks:                     {}",
        result.work.temporal_checks
    );
}

fn main() {
    println!("Pulse Field Lab");
    println!("EZ-013 — Bidirectional Heterogeneous Freshness / Global-TTL Tradeoff");
    println!();

    let (public_cache, pulse_cache) = build_slot_100_cache();
    assert_eq!(public_cache.len(), total_candidate_states());
    assert_eq!(pulse_cache.len(), total_candidate_states());
    assert_cache_integrity(&public_cache);

    let fast_states = public_cache
        .iter()
        .filter(|entry| is_fast_region(entry.transformation_id))
        .count();
    let slow_states = public_cache
        .iter()
        .filter(|entry| is_slow_region(entry.transformation_id))
        .count();
    let time_sensitive_states = fast_states + slow_states;
    let unaffected_states = total_candidate_states() - time_sensitive_states;

    let cached_sensitive_advance = public_cache
        .iter()
        .filter(|entry| {
            is_time_sensitive(entry.transformation_id) && entry.decision == Decision::Advance
        })
        .count();
    let cached_sensitive_reject = public_cache
        .iter()
        .filter(|entry| {
            is_time_sensitive(entry.transformation_id) && entry.decision == Decision::Reject
        })
        .count();

    assert_eq!(fast_states, 1_000);
    assert_eq!(slow_states, 2_000);
    assert_eq!(time_sensitive_states, 3_000);
    assert_eq!(unaffected_states, 7_000);
    assert_eq!(cached_sensitive_advance, 1_500);
    assert_eq!(cached_sensitive_reject, 1_500);

    let mut global_total = AggregateResult::default();
    let mut byte_only_total = AggregateResult::default();
    let mut ttl_20_total = AggregateResult::default();
    let mut ttl_50_total = AggregateResult::default();
    let mut cached_advance_total = AggregateResult::default();
    let mut cached_reject_total = AggregateResult::default();
    let mut pulse_total = AggregateResult::default();

    let mut oracle_advances = Vec::with_capacity(CHECKPOINTS.len());

    for current_protocol_slot in CHECKPOINTS {
        let oracle = run_oracle(current_protocol_slot);
        let oracle_advance = advance_count(&oracle);
        oracle_advances.push(oracle_advance);

        let (global, global_work) = run_global_recompute(current_protocol_slot);
        let (byte_only, byte_only_work) = run_byte_only(&public_cache);
        let (ttl_20, ttl_20_work) = run_global_ttl(
            &public_cache,
            current_protocol_slot,
            GLOBAL_TTL_20_VALID_UNTIL_SLOT,
        );
        let (ttl_50, ttl_50_work) = run_global_ttl(
            &public_cache,
            current_protocol_slot,
            GLOBAL_TTL_50_VALID_UNTIL_SLOT,
        );
        let (cached_advance, cached_advance_work) =
            run_cached_advance_only(&public_cache, current_protocol_slot);
        let (cached_reject, cached_reject_work) =
            run_cached_reject_only(&public_cache, current_protocol_slot);
        let (pulse, pulse_work) = run_pulse(&pulse_cache, current_protocol_slot);

        let global_errors = mismatch_counts(&oracle, &global);
        let byte_only_errors = mismatch_counts(&oracle, &byte_only);
        let ttl_20_errors = mismatch_counts(&oracle, &ttl_20);
        let ttl_50_errors = mismatch_counts(&oracle, &ttl_50);
        let cached_advance_errors = mismatch_counts(&oracle, &cached_advance);
        let cached_reject_errors = mismatch_counts(&oracle, &cached_reject);
        let pulse_errors = mismatch_counts(&oracle, &pulse);

        assert_eq!(global, oracle);
        assert_eq!(ttl_20_errors.false_authorizations, 0);
        assert_eq!(ttl_20_errors.false_rejects, 0);
        assert_eq!(pulse, oracle);

        if current_protocol_slot == 120 {
            assert_eq!(pulse_work.exact_recomputations, 0);
        }

        if current_protocol_slot == 121 {
            assert_eq!(pulse_work.exact_recomputations, 1_000);
        }

        if current_protocol_slot == 180 {
            assert_eq!(pulse_work.exact_recomputations, 1_000);
        }

        if current_protocol_slot == 181 {
            assert_eq!(pulse_work.exact_recomputations, 3_000);
        }

        println!("slot {current_protocol_slot}");
        println!("  Oracle ADVANCE:                       {oracle_advance}");
        println!(
            "  byte-only false auth / false reject:  {} / {}",
            byte_only_errors.false_authorizations, byte_only_errors.false_rejects
        );
        println!(
            "  TTL-20 exact / errors:                {} / {}",
            ttl_20_work.exact_recomputations,
            ttl_20_errors.false_authorizations + ttl_20_errors.false_rejects
        );
        println!(
            "  TTL-50 exact / errors:                {} / {}",
            ttl_50_work.exact_recomputations,
            ttl_50_errors.false_authorizations + ttl_50_errors.false_rejects
        );
        println!(
            "  Pulse exact / errors:                 {} / {}",
            pulse_work.exact_recomputations,
            pulse_errors.false_authorizations + pulse_errors.false_rejects
        );
        println!();

        add_work(&mut global_total.work, global_work);
        add_errors(&mut global_total.errors, global_errors);

        add_work(&mut byte_only_total.work, byte_only_work);
        add_errors(&mut byte_only_total.errors, byte_only_errors);

        add_work(&mut ttl_20_total.work, ttl_20_work);
        add_errors(&mut ttl_20_total.errors, ttl_20_errors);

        add_work(&mut ttl_50_total.work, ttl_50_work);
        add_errors(&mut ttl_50_total.errors, ttl_50_errors);

        add_work(&mut cached_advance_total.work, cached_advance_work);
        add_errors(&mut cached_advance_total.errors, cached_advance_errors);

        add_work(&mut cached_reject_total.work, cached_reject_work);
        add_errors(&mut cached_reject_total.errors, cached_reject_errors);

        add_work(&mut pulse_total.work, pulse_work);
        add_errors(&mut pulse_total.errors, pulse_errors);
    }

    assert_eq!(
        oracle_advances,
        vec![1_500, 1_500, 1_500, 1_500, 1_500, 1_500]
    );

    assert_eq!(global_total.errors.false_authorizations, 0);
    assert_eq!(global_total.errors.false_rejects, 0);
    assert_eq!(global_total.work.exact_recomputations, 60_000);

    assert_eq!(byte_only_total.errors.false_authorizations, 3_000);
    assert_eq!(byte_only_total.errors.false_rejects, 3_000);
    assert_eq!(byte_only_total.work.exact_recomputations, 0);
    assert_eq!(byte_only_total.work.cache_reuses, 60_000);

    assert_eq!(ttl_20_total.errors.false_authorizations, 0);
    assert_eq!(ttl_20_total.errors.false_rejects, 0);
    assert_eq!(ttl_20_total.work.exact_recomputations, 12_000);
    assert_eq!(ttl_20_total.work.cache_reuses, 48_000);

    assert_eq!(ttl_50_total.errors.false_authorizations, 500);
    assert_eq!(ttl_50_total.errors.false_rejects, 500);
    assert_eq!(ttl_50_total.work.exact_recomputations, 9_000);
    assert_eq!(ttl_50_total.work.cache_reuses, 51_000);

    assert_eq!(cached_advance_total.errors.false_authorizations, 0);
    assert_eq!(cached_advance_total.errors.false_rejects, 3_000);
    assert_eq!(cached_advance_total.work.exact_recomputations, 6_000);

    assert_eq!(cached_reject_total.errors.false_authorizations, 3_000);
    assert_eq!(cached_reject_total.errors.false_rejects, 0);
    assert_eq!(cached_reject_total.work.exact_recomputations, 6_000);

    assert_eq!(pulse_total.errors.false_authorizations, 0);
    assert_eq!(pulse_total.errors.false_rejects, 0);
    assert_eq!(pulse_total.work.exact_recomputations, 6_000);
    assert_eq!(pulse_total.work.cache_reuses, 54_000);
    assert_eq!(pulse_total.work.temporal_checks, 18_000);

    let reduction_vs_safe_global = 100.0
        * (ttl_20_total.work.exact_recomputations - pulse_total.work.exact_recomputations) as f64
        / ttl_20_total.work.exact_recomputations as f64;
    let reduction_vs_global_recompute = 100.0
        * (global_total.work.exact_recomputations - pulse_total.work.exact_recomputations) as f64
        / global_total.work.exact_recomputations as f64;

    assert!((reduction_vs_safe_global - 50.0).abs() < f64::EPSILON);
    assert!((reduction_vs_global_recompute - 90.0).abs() < f64::EPSILON);

    println!("fixture");
    println!(
        "  total candidate states:               {}",
        total_candidate_states()
    );
    println!("  fast-lifetime states:                 {fast_states}");
    println!("  slow-lifetime states:                 {slow_states}");
    println!("  unaffected states:                    {unaffected_states}");
    println!("  observed slot:                        {OBSERVED_SLOT}");
    println!("  fast valid through:                   {FAST_VALID_UNTIL_SLOT}");
    println!("  slow valid through:                   {SLOW_VALID_UNTIL_SLOT}");
    println!(
        "  cached sensitive ADVANCE / REJECT:    {cached_sensitive_advance} / {cached_sensitive_reject}"
    );
    println!();

    print_policy_result("Global recompute", &global_total);
    println!();
    print_policy_result("Byte-only reuse", &byte_only_total);
    println!();
    print_policy_result("Global TTL-20", &ttl_20_total);
    println!();
    print_policy_result("Global TTL-50", &ttl_50_total);
    println!();
    print_policy_result("Cached-ADVANCE-only challenger", &cached_advance_total);
    println!();
    print_policy_result("Cached-REJECT-only challenger", &cached_reject_total);
    println!();
    print_policy_result("Pulse evidence-specific freshness", &pulse_total);
    println!();

    println!(
        "Pulse exact reduction vs safe Global TTL-20: {:.2}%",
        reduction_vs_safe_global
    );
    println!(
        "Pulse exact reduction vs Global recompute:  {:.2}%",
        reduction_vs_global_recompute
    );
    println!();
    println!("EZ-013 CORRECTNESS GATE: PASS");
    println!(
        "ONE CLOCK DOES NOT FIT ALL EVIDENCE. WHEN DIFFERENT DECISION-RELEVANT FACTS EXPIRE AT DIFFERENT TIMES, A UNIVERSAL FRESHNESS HORIZON MUST EITHER RECOMPUTE STILL-VALID EVIDENCE OR RISK STALE DECISIONS; EVIDENCE-SPECIFIC VALIDITY MAY AVOID THAT TRADEOFF."
    );
}
