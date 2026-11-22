# Where this comes from

`/brag-slim` from https://github.com/latent-spaces/brag (MIT, see `LICENSE`), at commit
`06379432a9243c951dfcc051153be18d2d261044` (plugin version 0.6.0): `skills/brag-slim/SKILL.md`, copied
as it is (#1711) except for:

- the name, `brag`, and the title and usage line with it, so `/brag` runs it;
- the description's last sentence, which handed `/brag`'s phrases to the full skill; here this is the
  only one, so it takes them, and a trailer of the game, a patch or a card set besides;
- the section at the end, **In JackiOh**, which is this repository's own.

Upstream's full `/brag` is not here: it renders through Hyperframes and ships music, sound effects and
Python scripts, and a skill in `.claude/skills/` holds only Markdown and a licence
(`bot/go/tests/skills_test.go`). Upstream's own `/brag` hands off to `/brag-slim` on the newest models
anyway.

To update, copy a newer `skills/brag-slim/SKILL.md` over this one, make the same changes, keep the
section at the end, and change the commit and version above.
