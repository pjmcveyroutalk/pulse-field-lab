# Pulse Field Lab

Pulse Field Lab is an isolated experimental research environment for testing the Pulse Field architecture.

It is not Scout production code.

Its purpose is to falsify, measure, and refine architectural hypotheses before any concept is considered for promotion into a production system.

## Core Research Principle

Build the smallest machine capable of proving the idea wrong.

An idea does not survive because it is elegant.

It survives because measurement fails to kill it.

## Isolation Boundary

Pulse Field Lab has:

- no wallet authority
- no private keys
- no transaction signing
- no transaction submission
- no production capital
- no production execution authority

Pulse Field may eventually observe recorded or live economic state, but observation does not grant execution authority.

Scout and Pulse Field remain separate systems.

No Pulse Field capability becomes a Scout capability merely because an experiment succeeds.

## Experimental Discipline

Every experiment must define:

1. the question being tested
2. the reference or baseline
3. the treatment
4. the information available to each
5. the measurements
6. the correctness requirements
7. the failure conditions
8. the conditions required to continue

Experiments must preserve causal honesty.

Future information cannot be supplied to a decision that occurred before that information became knowable.

## Authority Rule

Learned intelligence is not execution authority.

Prediction may prioritize investigation.

Evidence may justify deeper investigation.

Proof may eventually authorize action in a future execution-capable system.

Pulse Field Lab itself has no execution authority.

## Experiment Zero

The first experiment is:

**EZ-M0 — Compression Without Loss**

Its first phase is:

**EZ-M0A — Representation Compression**

Question:

> Can Candidate Families preserve the baseline's economically important decisions while requiring less explicit representation and evaluation?

M0A deliberately excludes:

- machine learning
- prediction
- RPC
- live Solana state
- wallet logic
- transaction construction
- selective information acquisition
- Safe Ignorance
- Resolution Planning
- temporal readiness
- production integration

The experiment isolates one architectural hypothesis:

**Candidate-family compression.**

See `EXPERIMENT_ZERO.md` for the frozen experimental contract.

## Development Rule

Implementation follows experiment design.

No feature enters the prototype unless it answers a specific experimental question.

Complexity must pay rent.
