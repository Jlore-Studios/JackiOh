<!-- version: 1 -->
# Reconcile fullsend tree #$number

You are the reconciler for issue #$number of `$repo`, on branch `$branch` (base `$base`). A person
asked for this issue to be built with **fullsend**: it was split into parts, sub-issues that each
owned their files, and different agents built them, each blind to the others, onto this branch
instead of `main`, with no checks and no review. Every part has closed. Your run turns the branch
into one change that ships: it merges what was left aside, makes every check green, and opens one
pull request into `main`. The skill is in this worktree at `.claude/skills/fullsend/`.

## The task

$thread

Build what the issue asks for. Its title, body and the comments from trusted people above are
the request; later comments refine or override earlier ones.

$children

## The branch so far

$branch_state

$previous

## How to reconcile it

1. **Merge the parts left aside.** A part that conflicted with the branch when it landed was kept
   on a branch of its own instead. Merge each one below, in this order, before the next: `git
   merge --no-ff <branch>`, and when it conflicts, resolve every file, `git add` it and `git
   commit --no-edit`. These merges are the only commits you make: that overrides the system
   prompt's "do not commit" for them alone, and nothing else (never push, rebase or switch this
   worktree's branch). The harness commits the rest.

$parked

2. **Then follow `.claude/skills/fullsend/SKILL.md`, Phases 3 to 6**, in this one worktree. Read
   it in full first. Its Phases 0 to 2 are done: the issue is the spec, and the parts were the
   slices. Keep your scratch in `.fullsend/` (git ignores it here, and it never ships). This is
   not a fresh repository, so every phase works on what the parts added (`git diff
   $base...HEAD`): code that was on `main` before is not yours to delete or rewrite, beyond what
   the issue asks.
   - **Contact (Phase 3).** Build once (`cargo build --workspace --all-targets`, and `pnpm --dir
     apps/web typecheck` when the web changed) and classify the damage, fixing nothing yet:
     collisions, seams, drift and semantic conflicts, in `.fullsend/damage.md`.
   - **Reconcile (Phase 4).** For each collision pick one winner by the skill's score, delete
     every loser and update its callers, never merging two designs into one
     (`.claude/skills/fullsend/agents/reconciler.md`). Settle each semantic conflict against the
     issue and the spec, not against whichever part wrote more code.
   - **Green (Phase 5).** Compile first, then run the tests. When a test and the code disagree,
     the code loses by default, unless a line of the spec or the issue proves the test misread it;
     never edit a test to make it pass.
   - **Cull (Phase 6).** Delete what no test and no line of the issue reaches
     (`.claude/skills/fullsend/agents/culler.md`): delete, run the tests, and restore what they
     show was load-bearing.
3. **Green means this repository's checks**, not only a build: `cargo fmt --check`, `cargo clippy
   --workspace --all-targets -- -D warnings`, the tests of the crates you touched (`cargo test -p
   <crate>`), the web's test files you touched, `cargo jackioh spec check` when a ruling changed,
   and `cargo jackioh catalog check` and `cargo jackioh patches check` when a card did.
4. **Done when.** Check the issue's "Done when" (or what it asks for) against the branch, and build
   what is still missing: a part may have been cut short, or none of them owned it.

The project's rules in the system prompt hold for every agent you start: hand them on in each
brief, above all the paths no change may touch and the checks that may not be weakened.

## When you are done

The harness then commits your work, runs these checks and hands the whole diff against `main`,
every part's work included, to an adversarial reviewer who has not seen your reasoning. If the
reviewer finds problems, a fresh builder gets the findings, so leave nothing half-done.

$gate_list

If the task cannot be built without a person deciding something, stop and say so rather than
guessing.

## Your final message

The very first line must be a single HTML comment carrying JSON, with nothing before it:

    <!-- bot: {"status": "done", "title": "Pull request title, as docs/issues-and-patches.md (Titles) sets it"} -->

or, when you could not build it:

    <!-- bot: {"status": "blocked", "question": "The one question a person must answer, with the options you see"} -->

The title becomes the squash commit's subject, and a check holds it to the convention: usually
the issue's own title (`Patch v0.3.X: …`, `CI: …`, `Architecture: …`), with its version number
kept. A title that breaks the convention is replaced by the issue's.

After the first line, write the pull request description in Markdown: what changed and why, how you
tested it (the commands you ran and what they printed), every decision you made that the issue did
not settle with the alternative you rejected, and anything a reviewer should look at twice. Do not
restate the issue.
