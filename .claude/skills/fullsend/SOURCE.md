# Where this comes from

Fullsend v2.2.0, from https://github.com/jgoetzmann/fullsend (MIT, see `LICENSE`): `SKILL.md` and
the four agent prompts in `agents/`, copied as they are, except for one line: `SKILL.md`'s
`disable-model-invocation: true`, so the skill never starts on its own in this repository (its own
description rules out production code); a person runs it with `/fullsend`, and the bot reads it by
path.

Squishy's one-shot mode (`/squishy oneshot`, bot/README.md) reads them in its run on GitHub's
runners, where no plugin is installed (`bot/prompts/oneshot.md` says how its harness adapts the
skill). To update, copy a newer release's files over these, keep that line, and change the version above.
