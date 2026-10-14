---
name: maxi-tail
description: >
  The simplest change that actually works, held to JackiOh's rules. Ask whether the code needs to
  exist, reuse what the repo already has (the engine's prelude helpers and effect verbs, query.rs,
  a card with the same keyword, the testkit's scenario()), then the standard library, then a
  dependency already in the workspace, and only then write the minimum. Use on any coding task:
  writing, fixing, refactoring, reviewing or designing code, or choosing a dependency. Also use when
  someone says "maxi-tail", "be lazy", "simplest", "minimal", "yagni" or "do less", or complains
  about over-engineering, bloat or boilerplate. Not for non-coding questions.
license: MIT
---

# Maxi-tail

Lazy means efficient, not careless. The best code is the code never written; the next best is the
code this repo already has.

## The ladder

Stop at the first rung that holds:

1. **Does it need to exist?** A speculative need is skipped, with one line saying so.
2. **Is it already here?** Look before you write:
   - card scripts: the prelude's read helpers and the `jackioh_engine::effects` verbs (listed in
     `crates/cards/README.md`), and a card with the same keyword or trigger as a template;
   - board facts: `crates/engine/src/query.rs`; random pools: `crates/cards/src/query.rs`;
   - tests: the engine testkit's `scenario()` and its verbs;
   - the web: the generated types in `apps/web/src/wire/` and the typed functions over the WASM
     module, never a hand-written copy.

   A verb or fact the engine lacks goes into the engine with its own test (CLAUDE.md rule 5), not
   inline in one card.
3. **The standard library.**
4. **A platform feature:** CSS over JS, a database constraint over app code.
5. **A dependency already in the workspace.** Never a new one for what a few lines do; the pure
   crates take none (rule 4).
6. **One line**, if one line does it.
7. **Only then** the minimum new code.

The ladder runs after you understand the problem, not instead of it: read the task and the code it
touches, trace the real flow, then climb. When two rungs work, take the higher one.

**A bug fix is the root cause.** Grep every caller of the function before you edit it. One guard
in the shared function is a smaller diff than one in each caller, and patching only the path the
issue names leaves its siblings broken.

## Rules

- No abstraction nobody asked for: no trait with one implementation, no factory for one product, no
  option for a value that never changes. A named constant in the right `config.rs` is not one of
  these; rule 9 requires it.
- No scaffolding for later. Deletion over addition. Boring over clever.
- Fewest files and the shortest diff, in the right place: the smallest change in the wrong place is
  a second bug.
- A complex request: build the lazy version, say what it leaves out, and ask in the same reply.
  Never stall on a question you can default.
- Two options of the same size: take the one that is correct on edge cases.
- A corner cut on purpose with a known ceiling (an O(n²) scan, a simulation that ignores a case)
  gets a `maxi-tail:` comment naming the ceiling and the upgrade path, as the code already does.

## Never lazy about

- **The spec.** Implement what `spec/` says, never a simpler rule; a decision it does not make is a
  ruling with its test (rules 1 and 3).
- **The tests the repo requires.** A card's base and Radiant tests (rule 6), a ruling's named test
  (rule 3), and a unit test for any branch, loop or parser you add. Run the narrowest subset first
  (CLAUDE.md, Running a subset).
- **Purity, determinism and hidden information** (rules 4 and 7): every random draw through the
  state's `Rng`, every player choice a `PendingChoice`, nothing hidden in a view.
- **Trust boundaries:** validation in the server's API and match actor, security, data loss,
  accessibility, and anything explicitly asked for.
- **Understanding.** The ladder shortens the solution, never the reading.

## Output

Lead with the change, then at most three short lines: what was skipped and when to add it. A
report, walkthrough or pull request body that a person or a prompt asks for is given in full.

Off when someone says "stop maxi-tail" or "normal mode". Someone who insists on the full version
gets it, without re-arguing.
