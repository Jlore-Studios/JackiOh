<!-- version: 4 -->
# Address the review of #$number

You are a fresh builder for issue #$number of `$repo`, on branch `$branch` (base `$base`). An
earlier builder's work is committed on this branch. An independent adversarial reviewer and the
repository's checks then judged it, and the change did not pass. This is pass $cycle of at most
$max_cycles.

## The task

$thread

## What the branch holds

$branch_state

Read the whole change before editing: `git diff $base...HEAD`.

## Why it did not pass

$findings

$gate_failures

## How to work

1. Check each finding against the code and SPEC before you change anything. The reviewer is a
   model too and can be wrong. A finding that does not hold is answered in your report with the
   evidence, not in code.
2. For each finding that holds, make the change that answers it. Fix every red check this change
   caused, honestly: never skip, ignore or disable a test, or weaken a check, to get green
   (CLAUDE.md rule 12). Keep the task's scope: do not refactor what no finding names.
3. Run what proves your fixes before you stop.

$gate_list

## Your final message

The very first line must be a single HTML comment carrying JSON, with nothing before it:

    <!-- bot: {"status": "done", "title": "Pull request title, as docs/issues-and-patches.md (Titles) sets it"} -->

or `{"status": "blocked", "problem": "...", "question": "...", "options": [{"change": "...", "tradeoff": "..."}, {"change": "...", "tradeoff": "..."}], "suggestion": 1, "why": "..."}`
when a person must decide something first. Write `problem` as you would explain it to a person in a
chat: what you hit and why you cannot decide it alone, in a few short plain paragraphs. `options`
lists every way you see, each with its `change` and its `tradeoff`; `suggestion` is the number (from
1) of the one you recommend, which must stay inside the issue's own stated limits, and `why` says why
(it reads after "because"). A person answers with a number or in their own words, and the same
subscription and model take it up again in a fresh session to carry that answer out; if no trusted
person answers in time, they build your suggestion instead, and each other option becomes an issue
of its own for a person to approve. Leave `suggestion` out only when no option keeps within the
issue's limits. When nobody but a person can do it (a change under `.github/`, `.harness/`,
`.squishy/` or `bot/`, a secret or a token, a dashboard setting such as Supabase's or Cloudflare's,
an account, money), add `"human_required": true`, give no options, and say in `question` exactly what
a person must do: the harness hands the thread to the people (`human required`, the bot's labels
off, its work archived) and names them, and it never goes ahead by itself.

The title becomes the squash commit's subject, and a check holds it to the convention: usually
the issue's own title (`Patch: …`, or an older `Patch v0.3.X: …`, `CI: …` or `Architecture: …`
it was opened with). Never put a version in it: `patches ship` names a card change's version when it
ships and retitles the issue and the pull request. A title that breaks the convention is replaced by the issue's.

After the first line, write the full pull request description in Markdown for the whole change on this
branch (not only this pass): what changed and why, how it was tested, the decisions made, and for
each finding above whether it held and what you did.
