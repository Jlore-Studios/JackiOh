---
name: handoff
description: >
  Read and leave a handoff on a JackiOh issue or pull request, as CLAUDE.md rule 14 asks of every
  Claude session outside the bots' harness (Claude Code on the web, the desktop app, a cloud session
  a person started). Use before starting work on an issue or pull request (read the bot's journal
  and the handoffs already left), and at the end of every turn that pushes or leaves the work
  unfinished (write or update this session's own handoff comment). Also use when someone says
  "handoff", "leave a handoff", "pick up where it left off" or "what did the last session do".
  Not for the bots' own runs: the harness journals those.
---

# Handoff

A JackiOh item is worked by the night bot, Squishy and people's Claude sessions, often in turn. The
bots journal every run on the `bot-journal` branch (`<issue>.md`). A session cannot write that branch
(it pushes only its own), so it leaves its handoff as **one comment of its own** on the item, and the
bot's next run reads it there (`bot/go/internal/journal/handoffs.go`). This keeps one trail, readable
by the next session and the next bot run alike (#2748 section 10).

## Before you start: read

1. The item's journal, if it has one: `<issue>.md` on `bot-journal` (a pull request's runs are in the
   file of the issue it closes). With the GitHub tools, read the file at ref `refs/heads/bot-journal`;
   with git, `git fetch origin bot-journal` then `git show FETCH_HEAD:<issue>.md`. No file means no bot
   has run on it.
2. The handoff comments on the issue and on its pull request: comments holding
   `<!-- jackioh-session:handoff -->`. Read the newest few.
3. Pick up from **Left** and **Red**, and don't redo what **Tried and failed** already ruled out.

## Before you end a turn: write

At the end of every turn that pushed something, or that leaves the work unfinished, write this
session's handoff. Keep **one comment per session**: find the comment holding both the marker and
this session's link, and edit it in place; if there is none, create it. Put it on the issue, or on
the pull request when the work has no issue. Use this shape:

```markdown
### Handoff from a Claude session

- **Session:** <this session's link>
- **Branch / pull request:** <branch>, <pull request link, or "none yet">
- **Asked:** what the person asked for, in a line or two.
- **Done:** what this session did and pushed (commits, files), and what it verified.
- **Left:** what is still to do, in order, concrete enough that the next one starts there.
- **Tried and failed:** what was tried and why it didn't work, so nobody tries it again.
- **Red:** every check still red or not yet run, by its name (CLAUDE.md rule 12), or "none".

<!-- jackioh-session:handoff -->
```

Rules for the comment:

- Numbers, names, links and short facts only. No secrets, tokens, credentials or long logs.
- The bots never read a command, a vote or a mention from a handoff (`commands.CommandLines`), so a
  `/harness …` line in one does nothing. Write a command as its own comment when you mean it.
- Only trusted people's handoffs (`.harness/trust.txt`) reach a bot's run, the newest three, each cut
  to 6,000 characters (`journal.SessionHandoffKeep`, `SessionHandoffChars`). Put what matters first.
- End it with the repository's usual footer for a post written by Claude, if your session adds one.
  The marker may sit before it.
