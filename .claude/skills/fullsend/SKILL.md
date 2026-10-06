---
name: fullsend
description: |
  Build greenfield code fast by running many agents in parallel with the compiler
  turned off, then reconciling the wreckage against tests written from the spec.
  Use when the user says "fullsend", "just build it", "go fast", "don't overthink
  it", or "I don't care if it compiles yet." Use when starting a new project,
  prototype, spike, service, or rewrite from a written spec. Also use when a build
  is crawling because the agent keeps stopping to typecheck, or when the user
  complains about over-engineering, speculative abstraction, or too many small
  careful steps. Do not use on production code or an existing repo without an
  isolated worktree.
license: MIT
metadata:
  version: "2.2.0"
---

# Fullsend

Greenfield agent work is slow because it serializes twice: on the compiler, and
on other agents' half-finished interfaces. Write, typecheck, fix, write,
typecheck, fix — each stop costs wall clock, and the fix is usually to code
that gets deleted later in the run.

Fullsend removes both stops and pays once, at the end, mechanically.

- Agents write their whole slice blind — no compiler, no linter, no test run.
- They duplicate each other's work on purpose instead of coordinating.
- Test agents launch at the same moment, holding only the spec. No code exists
  yet, so they're blind to the implementation for free.
- The build stays broken from Phase 2 through Phase 4.
- Collisions get one winner and N deletions, scored, not debated.
- Green happens once, in Phase 5.
- Everything no test and no spec line reaches gets deleted unread.

Two ideas carry the weight. **Deletion beats coordination**: two agents writing
the same retry helper costs tokens; two agents coordinating so only one writes
it costs wall clock and blocks both, and wall clock is the scarce one.
**Overbuilding stops mattering when it's disposable**: speculative generality
is only expensive if someone maintains it, and Phase 6 deletes it unread.

## Use it when

All of these hold:

- New code, in a fresh repo, directory, or git worktree.
- A spec exists or can be written in Phase 0. Fullsend has no discovery
  phase; it converts a decided spec into code.
- The work splits into three or more chunks describable without reading each
  other.
- Nothing in the blast radius is live.

## Don't use it when

Any of these hold — work normally instead, and say why:

- The target is an existing production codebase you can't isolate.
- The task is one file or one function. Orchestration exceeds the work.
- The requirements are unknown and the point of the session is finding out.
  That's discovery; fullsend will confidently build the wrong thing eight times
  in parallel.
- It touches money movement, auth boundaries, PII, migrations, or infra other
  people depend on right now.
- The user wants to follow along. The middle of a run is unreadable.

## Setup

```bash
git switch -c fullsend/<feature>           # fresh repo: git init first
mkdir -p .fullsend/notes .fullsend/damage  # POSIX sh; {a,b} is bash-only
echo ".fullsend/" >> .gitignore
```

`.fullsend/` is scratch and never ships. Commit at every phase boundary
(`git commit -am "fullsend: phase N"`) so any phase rolls back without rerunning
the run. Say up front that Phase 2 runs silent and long, so the quiet reads as
expected rather than as a hang.

---

## Phase 0 — Spec and freeze (no agents)

The only phase where thinking is the point. Do it yourself. An ambiguity here
becomes eight divergent implementations.

Write `.fullsend/SPEC.md` with exactly these sections:

**`## Goal`** — one paragraph. What exists at the end that doesn't now.

**`## Surface`** — the interface freeze. Exact signatures for every cross-slice
boundary: routes, exported functions, events, table columns. Real types, real
argument order, real return shapes.

```
createOrder(userId: string, items: Item[]) -> Result<Order, OrderError>
Item = { sku: string, qty: int }
POST /orders  201 {order} | 422 {code, field} | 409 {code}
```

Precision here eliminates the largest class of Phase 3 seams: signature drift
is the failure parallelism causes most and reconcile fixes most slowly. Guess
where you must — a decided guess costs nothing, an open question costs one
divergent guess per builder.

**`## Behaviors`** — numbered, testable, one line each. `B7. POST /orders with
an empty cart returns 422 and writes nothing.` Aim for 15–40. These numbers are
the currency of the run: tests cite them, reconcile scores against them, cull
checks coverage of them.

**`## Out of scope`** — explicit list. This is what makes Phase 6 fast.

**`## Stack`** — language, framework, package manager, test runner, and the
exact dependency list with pinned versions. Agents that pick their own deps
produce three package manifests and a merge conflict.

Ask the user at most two questions about things you truly cannot decide. Then
freeze it. The spec does not change during the run; if it's wrong, that's the
next run's problem.

## Phase 1 — Shatter (no agents)

`slices = clamp(4, 10, ceil(behaviors / 4))`

Cut into **vertical** slices. Each owns its route, its logic, and its storage
access, top to bottom. Layered splits — one agent does all models, one does all
handlers — reintroduce exactly the blocking this skill exists to remove, and
they announce themselves in Phase 3 as one slice with more damage than all the
others combined.

Per slice, write a brief naming: the directory it exclusively owns, the behavior
numbers it owns (`B3, B4, B9`), and its public surface, copied verbatim from
`SPEC.md ## Surface`.

Leave overlaps in deliberately. If three slices need HTTP retry, don't factor it
out and don't assign it. All three write their own; Phase 4 keeps the best for
free.

Directory ownership is the only coordination mechanism in the entire skill.
Every file belongs to exactly one agent per wave, in every phase.

## Phase 2 — Full send (everything at once)

Launch in one batch:

- **One builder per slice** — `agents/builder.md`
- **Two to four spec-testers** — `agents/spec-tester.md`. One per 10 behaviors.

Testers start *now*, not after. No implementation exists, so they cannot see it
— blindness for free, and no possibility of tests that ratify whatever the code
happened to do. It is what makes RED safe and what gives Phase 5 its arbiter.

Builder rules, in priority order: never run the compiler, typechecker, linter,
or tests; stay inside your directory; match `## Surface` exactly at boundaries;
no stubs or TODOs; duplicate freely; stop at the limit. Pass
`agents/builder.md` verbatim, rule order intact — the order is why rule 1 holds.

Don't supervise. Watching tempts mid-flight correction, which restores the
serialization you just paid to remove.

## Phase 3 — Contact (no agents)

First build. **Fix nothing.** Classify.

```bash
<build command> 2>&1 | tee .fullsend/damage/raw.txt
```

Hundreds of errors is the phase working. Then run the two mechanical checks.

**Duplicate symbols** — names declared in more than one file, with the count:

```bash
kw='function|func|fn|def|class|type|struct|interface'
grep -rEo "($kw) +[A-Za-z_][A-Za-z0-9_]*" src/ \
  | awk -F: '{split($NF, w, " "); print w[2] "\t" $1}' \
  | sort -u | cut -f1 | uniq -c | sort -rn | awk '$1 > 1'
```

**Semantic conflicts** — the ones that compile fine and still disagree. Builders
answer a fixed ten-key list, so this is a diff, not a reading exercise:

```bash
cat .fullsend/notes/*.assumptions | sort -u | cut -d: -f1 | uniq -c | awk '$1>1'
```

Every key it prints is a conflict — two builders answered it differently; no
output means all ten agree. `grep '^<key>:' .fullsend/notes/*.assumptions` names
who chose what. Cents versus dollars, throw versus `null`, epoch versus RFC3339
never reach compiler output, and this is the last cheap place to catch them.

Write `.fullsend/damage.md` with four lists: **Collisions** (concept, every
location), **Seams** (imports pointing at nothing, or at a different signature),
**Drift** (dependency and config disagreement), **Semantic conflicts** (key,
competing values, owning slices).

Under twenty errors is a bad sign, not a good one: the agents ran the compiler
despite being told not to, so you paid the coordination cost and got none of the
parallelism. `grep -h BUILDS-RUN .fullsend/notes/*.md` — every line must read 0.

## Phase 4 — Reconcile

One reconciler per collision cluster — `agents/reconciler.md`.

**Wave packing.** Build a file→cluster map. Greedily pack clusters with disjoint
file sets into the same wave; anything overlapping goes to the next wave. Two
reconcilers must never hold the same file at the same time. Two waves is
typical.

**Winner scoring.** Mechanical, so nobody argues:

```
score = 10 × behaviors from SPEC.md satisfied
      −  3 × exported config options
      −  2 × type parameters
      −  1 × branches
tie → fewer lines → more existing callers
```

Simplicity is weighted negative on purpose. Nothing here has a future beyond
this run, so "more extensible" is not a tiebreaker.

**Pick one winner, delete every loser, update callers.** Never merge. Merging is
how a 40-line retry helper becomes a 200-line retry framework, and it costs
about four times what deleting costs. The tell is `git diff --shortstat` against
the Phase 3 commit showing more insertions than deletions.

Resolve each semantic conflict against `SPEC.md`, not against whichever agent
wrote more code. Record every decision in
`.fullsend/notes/reconcile-decisions.md`.

Still don't chase green. Error count often rises here as deletions expose more
seams. That's correct.

## Phase 5 — Green

Now the build matters. Bucket remaining errors by file cluster, one fix agent
per bucket, directory ownership again preventing overlap.

Compile first, then tests. Both at once produces agents that comment out failing
tests to make the build pass.

Two rules, non-negotiable:

- **A failing test is not automatically wrong.** It was written from the spec by
  something that never saw this code, so when a test and the implementation
  disagree, **the implementation loses by default.** Overriding requires quoting
  the `SPEC.md` line proving the test misread it, logged to
  `.fullsend/notes/test-corrections.md`.
- **Never edit a test to make it pass.** Rewriting a test is a spec question,
  escalated to the orchestrator, never an agent decision.

Two iterations is typical, three is tolerable. A fourth means the spec has
a hole — fix the spec,
then the code.

## Phase 6 — Cull

Green build, passing tests, and a lot of code nobody asked for. Deleting it is
how fullsend recovers the cost of having permitted overbuilding.

One culler — `agents/culler.md`. It works in order of lines removed per unit
of thought: anything under **Out of scope** that got built anyway first, then
unreferenced exports, single-implementation abstractions, config options whose
every call site passes the default, unreachable branches, scratch, and
duplicate test fixtures.

**Delete first, then run the tests.** The suite is the oracle: load-bearing code
fails on the next run and gets restored from git, which beats reading it to
decide. Restore and move on — a failed deletion is not an investigation.

Finish by listing every behavior no test cites:

```bash
grep -rhoE 'B[0-9]+' <test dir> | sort -u > .fullsend/cited.txt
grep -ohE 'B[0-9]+' .fullsend/SPEC.md | sort -u | comm -23 - .fullsend/cited.txt
```

Everything it prints is a real gap. Write those tests now.

---

## Phases

| Phase | What | Agents |
|---|---|---|
| 0 | Spec and interface freeze | none |
| 1 | Shatter into vertical slices | none |
| 2 | Full send — builders + testers together | 6–14 |
| 3 | Contact, two greps, damage report | none |
| 4 | Reconcile by score, in waves | 1/cluster |
| 5 | Compile, then pass | 1/bucket |
| 6 | Cull | 1 |

Phases 0, 1 and 3 are yours. The rest is fan-out.

## Abort criteria

Stop the run and say so plainly if a row of the failure table fires twice, or if:

- **Phase 4 hits semantic conflicts `SPEC.md` can't settle.** Stop, put more
  work into the spec, resume at Phase 4. Much cheaper than continuing.
- **The user asks what's happening and the honest answer is "I don't know."**
  Fullsend is opaque by design; that's tolerable only while it's on track.
  Roll back to the last phase commit and finish normally.

Aborting mid-run is cheap because of the phase commits. Continuing a bad run is
not.

## Failure modes

| Tell | Cause | Fix |
|---|---|---|
| Under 20 errors at Phase 3 | Builders ran the compiler | Find them with `grep -h BUILDS-RUN`; rerun those slices, rule 1 first |
| One slice's damage dwarfs the rest | Slices were layered, not vertical | Re-shatter; rerun Phase 2 for those slices |
| Line count rises in Phase 4 | Reconcilers merged instead of choosing | Rerun the cluster with an explicit delete instruction |
| Tests pass on first run in Phase 5 | Testers saw the implementation | Regenerate in a fresh agent with no repo access |
| Fourth Phase 5 iteration | Spec has a hole | Patch `SPEC.md` from `.fullsend/notes/spec-gaps.md`, then rerun the bucket |
| Reconcile keeps reopening | Slices were too small | Merge the reopened clusters into one wave; fewer, larger slices next run |

## Guardrails

- Branch or worktree, always. Fullsend is destructive by design.
- Commit at every phase boundary.
- Never fullsend twice into the same tree without going green first — broken
  code from run one poisons run two's collision analysis.
- `.fullsend/` never ships.

## Subagents

Pass the file verbatim, plus `.fullsend/SPEC.md` and what that role needs. The
reasoning behind every rule below is in §5, "Writing the agent prompts", of
[BUILD-FULLSEND.md](https://github.com/jgoetzmann/fullsend/blob/main/docs/BUILD-FULLSEND.md).

- `agents/builder.md` — Phase 2 slice builders; add the slice brief
- `agents/spec-tester.md` — Phase 2 test writers, spec-only, no repo access
- `agents/reconciler.md` — Phase 4 collision resolvers; add the cluster from
  `.fullsend/damage.md` and the wave's exclusive file list
- `agents/culler.md` — Phase 6 deletion pass; add the `TESTS-IN:` line

**No subagents available?** Fullsend degrades but still helps. Do the slices
yourself in sequence and keep the three mechanics that don't need parallelism:
write the tests from `SPEC.md` before any implementation, don't run the build
until every slice is written, and keep reconcile and cull intact. Expect a
modest speedup, not the full fan-out win. Say so up front.
