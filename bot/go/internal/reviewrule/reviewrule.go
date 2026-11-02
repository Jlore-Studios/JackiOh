// Package reviewrule is bot/harness/review_rule.py.
//
// The review rule: which approvals a commit needs before it ships. The deliver job counts the
// votes against it (`deliver.py`), and the router uses it to pick a review run's reviewer
// (`plan.review_seat`), so the two cannot disagree.
//
// A commit ships when no model's rejection of it stands and its lane's final audit approved it
// (#725, #2748): one approval from a medium or strong model for a medium item, one strong model's
// (Opus) for a hard item, and either from a subscription other than the one whose run built the
// commit. The builder's own reviewer, in its run or in a fresh session on its own subscription,
// still reviews and its findings still count, but its approval does not ship the commit; another
// of Muse's lanes is the same subscription. Approvals count per review, never per family: each
// review run, and each run's own adversarial reviewer, casts one. A weak approval never ships
// anything; it is a finding round, not a vote. A commit whose votes were recorded before #2748
// names no builder subscription (`built_by`) and keeps the rule it was reviewed under: one strong
// approval, from any subscription (`Counted`, `Tiers`).
//
// A commit that met the rule is recorded as cleared. When `main` then leaves it with conflicts,
// the clearance carries to the revision that resolves them (`carried`), so no review run is
// needed again: the revision started from the cleared commit, changed nothing but the files the
// merge left conflicted, and its own run's adversarial reviewer (medium or strong) approved it
// (`deliver._carry`). A revision that changed anything more goes back under the rule above.
package reviewrule

import (
	"fmt"
	"slices"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Summary is the rule in one line, for prompts, comments and the status issue.
var Summary string = "its lane's final audit (a medium or strong model's approval for a medium item, " +
	"a strong model's for a hard one, from a subscription other than its builder's)"

// Shipping is the tiers whose approval ships an item of `difficulty`, weakest first (#2748): a
// medium or strong model's for a medium item, a strong one's for a hard item.
func Shipping(difficulty string) []string {
	if difficulty == "hard" {
		return []string{"strong"}
	}
	return []string{"medium", "strong"}
}

// Recorded is whether `votes` names the subscription whose run built its head (`built_by`, ""
// for a head no run of the bot's built), as votes written since #2748 do.
func Recorded(votes *pyjson.Object) bool {
	_, ok := votes.Get("built_by").(string)
	return ok
}

// Tiers is the tiers whose approval of this head would count, weakest first: `Shipping`, or for
// votes from before #2748 (`Recorded` false) the strong tier alone, the rule they were cast under.
func Tiers(votes *pyjson.Object, difficulty string) []string {
	if !Recorded(votes) {
		return []string{"strong"}
	}
	return Shipping(difficulty)
}

// FromOther is whether a review recorded in `votes` came from a subscription other than the one
// whose run built the head: its `provider` is not the votes' `built_by`. A head no run of the
// bot's built (`built_by` "") is every subscription's other.
func FromOther(votes *pyjson.Object, review *pyjson.Object) bool {
	builtBy := py.OrStr(votes.Get("built_by"), "")
	return builtBy == "" || py.OrStr(review.Get("provider"), "") != builtBy
}

// Counted is the tiers of the head's approvals that count toward the rule, one per review: those
// from a subscription other than the builder's (`FromOther`). Votes from before #2748, which name
// no builder subscription, count their strong approvals alone, from any subscription, as they
// did then (`Tiers`).
func Counted(votes *pyjson.Object, familyTier func(string) string) []string {
	found := []string{}
	reviews, isList := votes.Get("reviews").([]any)
	if !Recorded(votes) || !isList {
		for _, a := range Approvals(votes, familyTier) {
			if a[1] == "strong" {
				found = append(found, a[1])
			}
		}
		return found
	}
	for _, r := range pyjson.Objs(reviews) {
		if !FromOther(votes, r) {
			continue
		}
		tier := py.OrStr(r.Get("tier"), "")
		if tier == "" {
			tier = familyTier(py.OrStr(r.Get("family"), ""))
		}
		found = append(found, tier)
	}
	return found
}

// Approvals is each approval of the head as (family, tier), one per review. A record kept
// before reviews were counted one by one holds one per approving family, at its recorded tier or
// else its family's strongest (`family_tier`).
func Approvals(votes *pyjson.Object, familyTier func(string) string) [][2]string {
	found := [][2]string{}
	if reviews, ok := votes.Get("reviews").([]any); ok {
		for _, r := range pyjson.Objs(reviews) {
			family := py.OrStr(r.Get("family"), "")
			tier := r.Get("tier")
			shown := ""
			if pyjson.Truthy(tier) {
				shown = py.Str(tier)
			} else {
				shown = familyTier(family)
			}
			found = append(found, [2]string{family, shown})
		}
		return found
	}
	tiers := votes.Obj("tiers")
	for _, f := range py.Each(votes.Get("approvals")) {
		var tier any
		if key, ok := f.(string); ok {
			tier = tiers.Get(key)
		}
		shown := ""
		if pyjson.Truthy(tier) {
			shown = py.Str(tier)
		} else {
			shown = familyTier(py.Str(f))
		}
		found = append(found, [2]string{py.Str(f), shown})
	}
	return found
}

// Met is whether approvals at these tiers (one per review, those that count: `Counted`) let an
// item of `difficulty` ship: one medium or strong approval for a medium item, one strong
// approval for a hard item (#2748). A weak approval never ships anything.
func Met(tiers []string, difficulty string) bool {
	for _, t := range tiers {
		if slices.Contains(Shipping(difficulty), t) {
			return true
		}
	}
	return false
}

// Carried is whether the head carries a cleared commit's clearance across a conflict resolution,
// and its own run's reviewer approved it: that one approval is enough.
func Carried(votes *pyjson.Object) bool {
	return votes.Truthy("carried") && votes.Truthy("approvals")
}

// Reachable is whether reviewers of the `available` tiers (on subscriptions other than the
// builder's) could still take the head to the rule: a strong one always can, and a medium one
// can for a medium item (#2748).
func Reachable(tiers []string, available []string, difficulty string) bool {
	if Met(tiers, difficulty) {
		return true
	}
	return Met(available, difficulty)
}

// Missing is what the head still waits for, in words (`Tiers`).
func Missing(votes *pyjson.Object, difficulty string) string {
	switch {
	case !Recorded(votes):
		return "one strong model's final audit (Opus)"
	case difficulty == "hard":
		return "one strong model's approval (Opus) from a subscription other than its builder's"
	}
	return "one medium or strong model's approval from a subscription other than its builder's"
}

// Who is "`gemini` (medium), `muse` (medium) twice": the approvals so far, a family's repeats
// together.
func Who(approved [][2]string) string {
	var order [][2]string
	count := map[[2]string]int{}
	for _, a := range approved {
		if _, ok := count[a]; !ok {
			order = append(order, a)
		}
		count[a]++
	}
	parts := make([]string, len(order))
	for i, a := range order {
		times := ""
		switch n := count[a]; n {
		case 1:
		case 2:
			times = " twice"
		default:
			times = fmt.Sprintf(" %d times", n)
		}
		parts[i] = fmt.Sprintf("`%s` (%s)%s", a[0], a[1], times)
	}
	return strings.Join(parts, ", ")
}
