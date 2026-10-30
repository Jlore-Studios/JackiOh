// Package reviewrule is bot/harness/review_rule.py.
//
// The review rule: which approvals a commit needs before it ships. The deliver job counts the
// votes against it (`deliver.py`), and the router uses it to pick a review run's reviewer
// (`plan.review_seat`), so the two cannot disagree.
//
// A commit ships when no model's rejection of it stands and its lane's final audit approved
// it (#725): one strong model's approval (Opus) for a medium or hard item, one medium
// model's for an easy item, whose scope the easy rule hard-limits. A strong approval ships
// an easy item too. Approvals count per review, never per family: each review run, and each
// run's own adversarial reviewer, casts one. A weak approval never ships anything; it is a
// finding round, not a vote.
//
// A commit that met the rule is recorded as cleared. When `main` then leaves it with conflicts,
// the clearance carries to the revision that resolves them (`carried`), so no review run is
// needed again: the revision started from the cleared commit, changed nothing but the files the
// merge left conflicted, and its own run's adversarial reviewer (medium or strong) approved it
// (`deliver._carry`). A revision that changed anything more goes back under the rule above.
package reviewrule

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// SUMMARY = "its lane's final audit: a strong model's approval, or a medium model's for an easy item"
var Summary string = "its lane's final audit: a strong model's approval, or a medium model's for an easy item"

// Each approval of the head as (family, tier), one per review. A record kept before reviews
// were counted one by one holds one per approving family, at its recorded tier or else its
// family's strongest (`family_tier`).

func Approvals(votes *pyjson.Object, familyTier func(string) string) [][2]string {
	panic("unported: review_rule.approvals")
}

// Whether approvals at these tiers (one per review) let an item of `difficulty` ship:
// one strong approval for a medium or hard item, one medium (or strong) approval for an
// easy item (#725: the Medium audit ships Easy). A weak approval never ships anything.

func Met(tiers []string, difficulty string) bool {
	panic("unported: review_rule.met")
}

// The head carries a cleared commit's clearance across a conflict resolution, and its own
// run's reviewer approved it: that one approval is enough.

func Carried(votes *pyjson.Object) bool {
	panic("unported: review_rule.carried")
}

// Whether one more approval at `tier` brings the head closer to the rule. A strong one
// always does; a medium one only where it ships (an easy item's final audit); a weak one
// never does (#725: weak self-review rounds are findings, and no weak review runs go out).

func Helps(tiers []string, tier string, difficulty string) bool {
	panic("unported: review_rule.helps")
}

// Whether reviewers of the `available` tiers could still take the head to the rule: a
// strong one always can, and a medium one can for an easy item (#725).

func Reachable(tiers []string, available []string, difficulty string) bool {
	panic("unported: review_rule.reachable")
}

// What the head still waits for, in words: a strong model's final audit, or a medium
// model's for an easy item (#725).

func Missing(tiers []string, difficulty string) string {
	panic("unported: review_rule.missing")
}

// "`gemini` (medium), `muse` (medium) twice": the approvals so far, a family's repeats
// together.

func Who(approved [][2]string) string {
	panic("unported: review_rule.who")
}
