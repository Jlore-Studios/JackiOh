// Package easy is bot/harness/easy.py.
//
// The easy rule: what `difficulty:easy` means, so that what a weak model may build is what it
// finishes (#317, "The easy rule").
//
// The weak tier may build only easy items. On this repository its model could not resolve the
// conflicts in SPEC §11 and the rulings index (every pull request appended the next R number there;
// since v0.3.0 a ruling is a note in `spec/rulings/`, listed in the generated `spec/INDEX.md`), could
// not hold a big change together, could not plan, ran no tests where it worked, and could not judge
// what it could not see. Its clean merges were all small (7–15 files, under about 500 lines).
//
// The rule is in the planner's prompt (`RULE`, `bot/prompts/plan.md`), and the harness holds the
// rating to it twice, trusting nothing the model said: deliver checks the plan's "Files to touch"
// table before the item is queued (`plan_breach`), and the real change before a weak builder's work
// is pushed (`change_breach`). Either breach makes the item medium, and so does a plan or change
// close to a limit (borderline easy is medium, #695) or a rating whose `why` admits doubt
// (`uncertain_why`). The limits are settings (`easy` in `.harness/config.json`, `config.EasyRule`).
package easy

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// BORDERLINE_FILES = 2
var BorderlineFiles int = 2

// BORDERLINE_LINES = 100
var BorderlineLines int = 100

// Whether a planner's rating `why` admits doubt about easy (deliver then rates medium:
// borderline easy is medium). A confident `why` ("one file, one test") is untouched.

func UncertainWhy(why string) bool {
	panic("unported: easy.uncertain_why")
}

// RULE = """An item is **easy** only if every line below holds. If any line fails, or you cannot tell, it is **medium** at least. 1. **Small, with room to spare.*
var Rule string = "An item is **easy** only if every line below holds. If any line fails, or you cannot\ntell, it is **medium** at least.\n\n1. **Small, with room to spare.** At most $max_files files changed in all, the tests it adds\n  included, and at most $max_lines lines added plus removed (generated files such as\n  `apps/web/src/wire/generated/` aside). Close to either limit is **medium**, not easy.\n2. **One place.** All of it inside one crate or app, except a test of the same feature in that\n  crate's `tests/`.\n3. **None of the conflict hot spots:** `SPEC.md` and the spec's notes in `spec/` (no new ruling,\n  no ruling cited or renumbered, nothing `spec/INDEX.md` lists), `BUILD.md`.\n4. **None of the shared surfaces:** `crates/engine/src/wire/events.rs`, `crates/engine/src/script.rs`,\n  `crates/engine/src/state.rs`, `crates/engine/src/effects/mod.rs`.\n5. **No rules or data work:** nothing in `crates/engine/src/` or `crates/ai/src/`, no change to\n  `crates/cards/catalog.json`, `crates/cards/patches/` or the catalog version, no card script\n  (`crates/cards/src/scripts/`), no database migration (`crates/server/migrations/`).\n6. **No review-only or forbidden path** (every `package.json` and `Cargo.toml`, the lockfiles,\n  `rust-toolchain.toml`, the build and test configs, `scripts/`, `vercel.json`, `render.yaml`;\n  `.github/`, `.harness/`, `bot/`), and no new dependency.\n7. **Checked by a unit test** in the same crate or app, which the plan names; no Cypress or e2e\n  spec change, and no judging by eye (animation, layout, sound or art tuning).\n8. **No decisions left:** the plan names each file, what changes in it, and the test that proves\n  it. If the builder would choose between two designs, it is not easy.\n9. **Stands alone:** blocked by no open issue, and no open pull request touches the same files.\n\nNever easy: a patch that changes cards or rules, anything that adds an R number, engine or AI\nwork, visual polish.\n\n- **medium:** anything that fails the easy rule and is not hard: a new ruling, a card or catalog\n change, a shared surface, several crates or apps, e2e work, visual polish.\n- **hard:** engine rules or the resolution loop (`crates/engine/src/`), the AI's search\n (`crates/ai/src/`), design across crates, a database migration, or about 40 files or more.\n\nWhen torn between two, choose the higher. A borderline easy is **medium**: any doubt on\nany line above rates **medium** at least."

// The paths in the first column of the plan's "Files to touch" table, in order.
//
// A row's first cell may hold its path in backticks or bare; a header row, the separator row and
// a cell that is not a path (no `/` or `.`) are skipped.

func PlanPaths(plan string) []string {
	panic("unported: easy.plan_paths")
}

// Why a plan rated easy breaks the rule (lines 1, 3–6, from its "Files to touch" table), or
// "" when it does not. A plan with no such table cannot show it holds, so it breaks line 8.
// A plan within `BORDERLINE_FILES` of the file limit is borderline easy, so medium (#695).

func PlanBreach(plan string, rule any, reviewPaths []string, forbiddenPaths []string) string {
	panic("unported: easy.plan_breach")
}

// Why a change built for an easy item breaks the rule (lines 1 and 3–5, from what it really
// changed: lines per path, `Git.numstat`), or "" when it does not. A change within
// `BORDERLINE_FILES` files or `BORDERLINE_LINES` lines of a limit is borderline easy, so
// medium (#695).

func ChangeBreach(counts *pyjson.Object, rule any) string {
	panic("unported: easy.change_breach")
}
