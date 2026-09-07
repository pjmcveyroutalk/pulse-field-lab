# Experiment Zero

## EZ-M0 — Compression Without Loss

### Phase

EZ-M0A — Representation Compression

### Status

Experimental contract frozen before implementation.

---

## 1. Research Question

Can Candidate Families preserve the economically important decisions produced by explicit enumeration while requiring less explicit representation and evaluation?

This experiment tests representation compression only.

It does not test whether Pulse Field can make money.

It does not test prediction.

It does not test execution.

It does not test Solana.

---

## 2. Survival Hypothesis

Candidate-family compression can reduce explicit computational work while preserving every determinate economically important decision produced by the reference semantics.

## 3. Null Outcome

Candidate-family compression does not materially reduce explicit work, cannot preserve the reference decisions, or introduces enough additional machinery that the apparent compression advantage disappears.

A null outcome is a valid research result.

The architecture must change if the evidence requires it.

---

## 4. Ground Truth

Each synthetic fixture contains a deterministic set of transformations.

Every transformation has sufficient ground-truth information to determine its correct economic decision.

Ground truth belongs to the fixture oracle.

The oracle is used for scoring only.

Neither engine may inspect hidden oracle state while making decisions.

---

## 5. Identical Information Rule

The Baseline Engine and Pulse Engine receive the same economically relevant visible information.

Pulse may use a different representation of that information.

Pulse may not receive additional facts unavailable to the baseline.

Compression is allowed.

Information advantage is not.

---

## 6. Baseline Engine

The baseline explicitly enumerates every transformation.

For each transformation it:

1. reads the applicable visible facts
2. evaluates the applicable constraints
3. computes the transformation's economic state
4. assigns a decision

The baseline intentionally does not use Candidate Families.

It establishes the explicit-enumeration reference.

---

## 7. Pulse Engine

Pulse begins with Candidate Families.

A Candidate Family represents multiple transformations that share economically meaningful structure.

Pulse may:

- evaluate shared structure once
- maintain valid economic bounds
- reject a family when valid shared structure proves no member can advance
- preserve a family without expanding its individual members
- split a family when members can no longer safely share a decision
- expand exact transformations when distinction becomes economically necessary

Pulse may not reject possibilities merely because they are inconvenient to compute.

### Governing rule

**Split where the decision changes, not merely where a variable changes.**

---

## 8. Decision Semantics

The experiment uses three decisions:

- `ADVANCE`
- `REJECT`
- `UNRESOLVED`

For an exact transformation:

### ADVANCE

The transformation satisfies all hard constraints and its proven net economic value is greater than the experiment threshold.

For M0A the threshold is:

`net_value > 0`

### REJECT

The transformation violates a hard constraint or its proven net economic value cannot exceed the threshold.

### UNRESOLVED

The available experiment state does not permit a valid ADVANCE or REJECT decision.

Because M0A begins with complete visible information, determinate fixtures should not normally finish UNRESOLVED.

Pulse cannot claim computational savings by remaining UNRESOLVED where the baseline correctly resolves a decision.

Such a result is a disagreement.

---

## 9. Economically Important Transformation

For M0A, an economically important transformation is any transformation whose oracle-ground-truth decision is `ADVANCE`.

This definition is intentionally narrow.

Future experiments may introduce richer economic objectives.

M0A does not.

---

## 10. Correctness Gate

Correctness precedes efficiency.

For determinate M0A fixtures:

**False Important Prunes must equal zero.**

Pulse must not remove a transformation whose ground-truth decision is ADVANCE.

The target decision agreement is:

**100%**

Any disagreement must be recorded and causally traceable.

Efficiency cannot compensate for incorrect decisions.

---

## 11. Work Accounting

M0A measures logical work directly.

Primary work measurements include:

- candidate-family evaluations
- exact candidate expansions
- constraint evaluations
- economic evaluations
- fact accesses
- other deterministic resolution operations introduced by either engine

Raw operation counts must be retained.

The first experiment will not hide these measurements inside an arbitrary weighted score.

Wall-clock runtime may also be recorded, but it is secondary because implementation details can distort small synthetic benchmarks.

### Governing rule

**Measure skipped work as carefully as performed work.**

---

## 12. Fixture Determinism

Every fixture must be reproducible.

A fixture records:

- fixture version
- deterministic seed where applicable
- transformation count
- family structure
- visible facts
- constraints
- ground truth
- experiment configuration

The same fixture must produce the same economic world.

---

## 13. Initial Fixture — EZ-001

EZ-001 intentionally gives Candidate Families a favorable environment.

This is deliberate.

If compression cannot produce a meaningful advantage under favorable structure, there is little reason to immediately test it under harder conditions.

Initial target world:

- 10,000 possible transformations
- strong shared structure
- approximately 7,000 transformations eliminable by one shared constraint
- approximately 2,000 additional transformations eliminable by another shared economic bound
- approximately 1,000 remaining possibilities
- only a small subset should require exact individual distinction

These are fixture design targets, not claimed experimental results.

The implementation must report the actual generated distribution.

---

## 14. Progressive Murder Tests

If EZ-001 survives, later fixtures attack the hypothesis.

Planned progression:

### EZ-002
Overlapping constraints.

### EZ-003
Quantity coupling.

### EZ-004
Stale evidence.

### EZ-005
Changing generations and ABA state transitions.

### EZ-006
Protocol-time changes without corresponding data-value changes.

### EZ-007
Dangerous false-merge conditions.

Each phase must isolate the architectural behavior being tested as much as practical.

---

## 15. Failure Conditions

The hypothesis fails or requires redesign if:

- compression silently removes economically important transformations
- Candidate Families cannot preserve reference decisions
- the machinery required for compression consumes the practical savings
- Pulse performs approximately the same explicit work while merely describing it differently
- favorable synthetic results collapse when realistic structure is introduced
- correctness depends on information unavailable to the baseline
- results depend on hindsight or future information

No success metric may excuse a correctness violation.

---

## 16. Efficiency Interpretation

M0A does not predeclare an arbitrary percentage improvement.

The first experiment measures the actual work reduction.

This prevents selecting a convenient threshold after implementation details are known.

If M0A demonstrates correctness and measurable compression, a later confirmatory experiment may freeze a material efficiency threshold before being run.

---

## 17. Causal Traceability

Every final decision must be explainable.

For each disagreement, the harness must identify the earliest point at which Baseline and Pulse reasoning diverged.

This is the:

**Divergence Point**

A disagreement without traceable ancestry is itself an experimental defect.

---

## 18. Exclusions

M0A does not include:

- wallet authority
- signing
- transaction submission
- capital
- production Scout integration
- live RPC
- Solana execution
- flash loans
- Jito
- venue adapters
- machine learning
- predictive signals
- Safe Ignorance
- Resolution Planning
- learned attention
- adaptive topology
- regime learning

Those concepts remain outside the experiment until evidence justifies introducing them.

---

## 19. Promotion Meaning

Passing M0A means only:

> Candidate-family representation preserved the tested decisions while reducing measured explicit work enough to justify further experimentation.

It does not prove Pulse Field.

It does not prove profitability.

It does not grant production authority.

It earns the next experiment.

---

## 20. Research Law

**The baseline tells us what is correct. Pulse must prove it can reach that answer with less work.**

And:

**The first implementation exists to falsify the architecture, not demonstrate it.**
