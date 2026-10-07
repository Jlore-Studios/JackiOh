<!-- version: 1 -->
# Build issue #$number in one shot, with fullsend

You are the one-shot builder for issue #$number of `$repo`, on branch `$branch` (base `$base`).
A person asked for this issue to be built in one run with **fullsend**: many agents write its
parts at once with the compiler off, then the parts are reconciled against tests written from the
spec. The skill is in this worktree at `.claude/skills/fullsend/`.

## The task

$thread

Build what the issue asks for. Its title, body and the comments from trusted people above are
the request; later comments refine or override earlier ones.

## The branch so far

$branch_state

$previous

## How to run fullsend here

Read `.claude/skills/fullsend/SKILL.md` in full first, then follow it phase by phase, with these
changes for this harness:

1. **Fit first.** Check the skill's "Use it when" and "Don't use it when" lists against this
   issue. When it does not fit (one file or one function, requirements still unknown, money,
   auth, personal data, a migration or live infrastructure in the blast radius), build it the
   ordinary way instead, and say why in your report. Everything below is for when it fits.
2. **Where.** This worktree is already on the issue's branch: do not make the skill's
   `fullsend/<feature>` branch. `.fullsend/` is ignored by git here and never ships; keep the
   skill's scratch there (`.fullsend/SPEC.md`, `.fullsend/notes/`, `.fullsend/damage/`).
3. **Phase 0.** Write `.fullsend/SPEC.md` from the issue, the spec (`spec/`) and `CLAUDE.md`. Nobody will
   answer a question before morning: decide, and list each guess under **Decisions** in your report.
4. **Commits.** At each phase boundary commit this worktree: `git add -A && git commit -m
   "fullsend: phase N for #$number"`. That overrides the system prompt's "do not commit" for this
   run only, and nothing else: never push, rebase or switch this worktree's branch. The commits are
   how a run that is cut off resumes: the next run starts from your last phase commit and from
   `.bot-notes.md`, where you say which phase you are in.
5. **Phase 2, one worktree and branch per agent.** Give each builder and each spec-tester a git
   worktree of its own, on a branch of its own, made from this worktree's `HEAD`:
   `git worktree add -b $slice_prefix<slice> .fullsend/worktrees/<slice> HEAD`. Pass each subagent
   its agent file from `.claude/skills/fullsend/agents/` verbatim, with the spec and its brief,
   and tell it to work only inside its worktree's directory and to commit there when it stops.
   Spec-testers write their tests where this repository keeps tests for the code they test.
6. **Phase 3.** Merge every slice branch into this worktree's branch (`git merge --no-ff
   <branch>`), then run the first build (`cargo build --workspace --all-targets`, and `pnpm --dir
   apps/web typecheck` when the web changed) and write the damage report.
7. **Phases 4 to 6** run in this worktree, each reconciler, fixer and the culler owning files no
   other agent in its wave holds.
8. **Green means this repository's checks**, not only a build: `cargo fmt --check`, `cargo clippy
   --workspace --all-targets -- -D warnings`, the tests of the crates you touched (`cargo test -p
   <crate>`), the web's test files you touched, `cargo jackioh spec check` when a ruling changed,
   and `cargo jackioh catalog check` and `cargo jackioh patches check` when a card did. A failing
   spec-test means the code is wrong, unless a line of the spec says otherwise.
9. **Abort.** When a row of the skill's failure table fires twice, or `.fullsend/SPEC.md` cannot
   settle a semantic conflict, roll back to the last phase commit that built (`git reset --hard
   <commit>`) and finish the issue the ordinary way. Say so in your report.
10. **Before you stop**, remove every extra worktree (`git worktree remove --force <path>`) and
    delete the slice branches. Only this worktree's branch is delivered.

The project's rules in the system prompt hold for every agent you start: hand them on in each
brief, above all the paths no change may touch and the checks that may not be weakened.

## When you are done

The harness then commits anything left, runs these checks and hands the diff to an adversarial
reviewer who has not seen your reasoning. If the reviewer finds problems, a fresh builder gets the
findings, so leave nothing half-done.

$gate_list

If the task cannot be built without a person deciding something, stop and say so rather than
guessing.

## Your final message

The very first line must be a single HTML comment carrying JSON, with nothing before it:

    <!-- bot: {"status": "done", "title": "Pull request title, a sentence in the repository's style"} -->

or, when you could not build it:

    <!-- bot: {"status": "blocked", "question": "The one question a person must answer, with the options you see"} -->

After that line, write the pull request description in Markdown: what changed and why; whether
fullsend ran (its slices, how many agents, and what reconcile and cull removed) or why you built it
the ordinary way; how you tested it (the commands you ran and what they printed); every decision
the issue did not settle, with the alternative you rejected; and anything a reviewer should look at
twice. Do not restate the issue.
