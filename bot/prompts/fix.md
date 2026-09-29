<!-- version: 1 -->
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
   caused. Keep the task's scope: do not refactor what no finding names.
3. Run what proves your fixes before you stop.

$gate_list

## Your final message

The very first line must be a single HTML comment carrying JSON, with nothing before it:

    <!-- bot: {"status": "done", "title": "Pull request title, a sentence in the repository's style"} -->

or `{"status": "blocked", "question": "..."}` when a person must decide something first.

After that line, write the full pull request description in Markdown for the whole change on this
branch (not only this pass): what changed and why, how it was tested, the decisions made, and for
each finding above whether it held and what you did.
