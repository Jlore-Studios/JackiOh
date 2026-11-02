// Package spotcheck is the after-merge spot-check (#2748, section 5).
//
// An item ships on one approval at its tier (a medium or strong model's for a medium item, a strong
// one's for a hard item), its builder's own lane's included (`reviewrule`, the owner's decision of
// 2026-11-02): most often the adversarial reviewer in the very run that built it. This check is
// the independent measure of that self-review: a strong model (Opus) on another subscription
// reviews a share of those changes once they have merged, every `spot_check_every`th medium or
// hard bot pull request merged into the default branch (5 for the night bot; 0, off, for
// Squishy). The share is a counter in the state file (`Key`'s `count`), never a draw, so whoever
// counts a merge counts it the same way. A fullsend tree's reconcile is never counted, and nor is a
// follow-up issue's own pull request, so a follow-up never samples itself (no loops). The check's
// record keeps who built the head and whose approval shipped it, so the usage history can tell a
// self-approved head from a cross-approved one.
//
// The check is a review run of the change as it merged: the commit that merged the pull request,
// against its first parent (`work.Worker.SecondReview`), with the review's prompt of its own
// (`bot/prompts/spotcheck.md`). It goes to a strong seat on a subscription other than the one that
// built the head, and other than the one whose approval shipped it when another is free
// (`plan.ReviewSeat`), after every other review run, since it unblocks nothing, and before builds
// (`plan.Pairs`). It blocks nothing and reverts nothing. Its verdict (`deliver.SpotCheck`): an
// approval records `clean`; blocking findings open one follow-up issue (`Title`, `Marker`),
// queued to build at the merged change's difficulty and linked from a comment on the pull request, and
// record `findings`. Every check stays in the state file with its pull request, the builder and
// the approver with their model families, the reviewer, the verdict, how many findings and the
// follow-up (`Kept` of them), for the usage history (#2749) to chart the miss rate per family.
package spotcheck

import (
	"fmt"
	"slices"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	hstate "github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
)

// Key is the state file's record of the spot-checks: `{"count": <medium merges counted>,
// "checks": [<one entry per check, oldest first>]}`.
var Key = "spot_checks"

// Kept is how many finished checks the state file keeps, the oldest dropped first: the state
// file must stay well under the 1 MB the Contents API reads, and the usage history (#2749) and
// the journal keep the record from there on.
var Kept = 100

// Tier is the tier a spot-check runs on: a strong model's (Opus).
var Tier = "strong"

// Difficulties is the difficulties whose merges are counted: both, since each ships on its own
// lane's approval (2026-11-02, #2748).
var Difficulties = []string{"medium", "hard"}

// FollowUpDifficulty is the difficulty a follow-up issue is queued at: the merged change's, or
// medium when it is not on record.
func FollowUpDifficulty(entry *pyjson.Object) string {
	if d := entry.Str("difficulty"); slices.Contains(Difficulties, d) {
		return d
	}
	return Difficulties[0]
}

// The verdicts a finished check records.
const (
	Clean    = "clean"    // the reviewer approved the merged change
	Findings = "findings" // it had blocking findings, which a follow-up issue holds
	Failed   = "failed"   // no run of it finished: too many tries, or nothing to review
)

// Title is the follow-up issue's title for pull request `pr`.
func Title(pr int) string {
	return fmt.Sprintf("Patch: follow-up to #%d from the after-merge spot-check", pr)
}

// markerPrefix opens the line a follow-up issue's body starts with (`Marker`).
var markerPrefix = "<!-- jackioh-bot:spot-check-follow-up"

// Marker is the first line of the follow-up issue of pull request `pr`, which says what it is
// even after its record is pruned.
func Marker(pr int) string { return fmt.Sprintf("%s #%d -->", markerPrefix, pr) }

// IsFollowUp is whether an issue is a spot-check's follow-up: its record says so (`spot_check_of`,
// which deliver writes when it opens one), or its body carries the marker.
func IsFollowUp(record *pyjson.Object, body string) bool {
	return py.MustInt(py.Or(record.Get("spot_check_of"), 0)) != 0 || strings.Contains(body, markerPrefix)
}

// Record is the state file's spot-check record, made when it has none.
func Record(s *pyjson.Object) *pyjson.Object {
	found, ok := s.Get(Key).(*pyjson.Object)
	if !ok || found == nil {
		found = pyjson.New("count", 0, "checks", []any{})
		s.Set(Key, found)
	}
	return found
}

// Checks is every check on record, oldest first.
func Checks(s *pyjson.Object) []*pyjson.Object {
	return pyjson.Objs(s.Obj(Key).Get("checks"))
}

// Find is the check of pull request `pr` that has not finished, or nil.
func Find(s *pyjson.Object, pr int) *pyjson.Object {
	checks := Checks(s)
	for i := len(checks) - 1; i >= 0; i-- {
		c := checks[i]
		if py.MustInt(py.Or(c.Get("pr"), 0)) == pr && c.Str("status") != "done" {
			return c
		}
	}
	return nil
}

// Pending is the checks waiting for a run: queued, and no run holds them.
func Pending(s *pyjson.Object) []*pyjson.Object {
	var found []*pyjson.Object
	for _, c := range Checks(s) {
		if c.Str("status") == "queued" && !c.Truthy("run_id") {
			found = append(found, c)
		}
	}
	return found
}

// Running is whether a run holds the check of pull request `pr`.
func Running(s *pyjson.Object, pr int) bool {
	found := Find(s, pr)
	return found != nil && found.Truthy("run_id")
}

// Merge is what a merged bot pull request brings to the counter.
type Merge struct {
	PR, Issue  int
	Title      string
	SHA        string // the commit that merged it into the default branch
	Head       string // its head when it merged
	Difficulty string
	// FollowUp: its issue is a spot-check's follow-up, which is never counted.
	FollowUp bool
	// Reconcile: its issue is a fullsend tree, so this is the tree's reconcile, never counted.
	Reconcile bool
	// Record is the pull request's item record: its votes (who built the head) and its
	// clearance (who approved it, `cleared`'s `approver`).
	Record *pyjson.Object
}

// builderOf is who built the head that merged, from its votes: `{"provider", "family"}`, or {}
// when no run of the bot's is on record for it.
func builderOf(m Merge) *pyjson.Object {
	votes := m.Record.Obj("votes")
	if votes == nil || (m.Head != "" && votes.Str("sha") != m.Head) {
		return pyjson.New()
	}
	return pyjson.New("provider", votes.Str("built_by"), "family", votes.Str("builder"))
}

// approverOf is whose approval shipped the head that merged (its clearance's `approver`, the
// merge owner): `{"provider", "family", "model", "tier"}`, or {} when no approval cleared it.
func approverOf(m Merge) *pyjson.Object {
	cleared := m.Record.Obj("cleared")
	if cleared == nil || (m.Head != "" && cleared.Str("sha") != m.Head) {
		return pyjson.New()
	}
	if approver := cleared.Obj("approver"); approver != nil {
		return approver.Copy()
	}
	return pyjson.New("provider", m.Record.Str("merge_owner"))
}

// Merged counts a bot pull request merged into the default branch (`events.OnPullClosed`), once:
// a medium or hard one that is no follow-up's or reconcile adds one to the count when the bot
// spot-checks at all (`every` above 0), and every `every`th is queued for a check. Returns the
// check it queued, or nil.
func Merged(s *pyjson.Object, m Merge, every int, at time.Time) *pyjson.Object {
	if every <= 0 || !slices.Contains(Difficulties, m.Difficulty) || m.FollowUp || m.Reconcile {
		return nil
	}
	item := hstate.Item(s, m.PR)
	if item.Has("spot_check_n") {
		return nil // counted already (an event delivered twice)
	}
	record := Record(s)
	count := py.MustInt(py.Or(record.Get("count"), 0)) + 1
	record.Set("count", count)
	item.Set("spot_check_n", count)
	if count%every != 0 {
		return nil
	}
	entry := pyjson.New("pr", m.PR, "issue", m.Issue, "title", py.SliceTo(strings.Join(py.Split(m.Title), " "), 200),
		"n", count, "every", every, "merged_at", clock.ISO(at), "sha", m.SHA, "head", m.Head,
		"difficulty", m.Difficulty, "builder", builderOf(m), "approver", approverOf(m),
		"status", "queued", "queued_at", clock.ISO(at), "tries", 0)
	record.Set("checks", append(pyjson.List(record.Get("checks")), entry))
	return entry
}

// Start records the run that takes the check of pull request `pr` up, and its reviewer
// (`{"provider", "family", "model", "tier"}`).
func Start(s *pyjson.Object, pr int, runID string, reviewer *pyjson.Object, at time.Time) {
	if found := Find(s, pr); found != nil {
		found.Update(pyjson.New("run_id", runID, "started_at", clock.ISO(at), "reviewer", reviewer))
	}
}

// Requeue puts the check of pull request `pr` back in the queue: its run ended without a verdict
// (`why`). With `limit` above 0 the run counts as a try, and the `limit`th try finishes it as
// `failed`. Whether it gave up.
func Requeue(s *pyjson.Object, pr int, why string, limit int, at time.Time) bool {
	found := Find(s, pr)
	if found == nil {
		return false
	}
	tries := py.MustInt(py.Or(found.Get("tries"), 0))
	if limit > 0 {
		tries++
	}
	found.Update(pyjson.New("run_id", "", "tries", tries, "last_why", py.SliceTo(why, 300)))
	if limit > 0 && tries >= limit {
		Finish(s, pr, pyjson.New("verdict", Failed, "findings", 0, "reason", py.SliceTo(why, 300)), at)
		return true
	}
	return false
}

// Finish records the verdict of the check of pull request `pr` (`verdict`, `findings`, its
// `follow_up` issue, …), and keeps the `Kept` newest finished checks.
func Finish(s *pyjson.Object, pr int, fields *pyjson.Object, at time.Time) {
	found := Find(s, pr)
	if found == nil {
		return
	}
	found.Update(fields)
	found.Set("status", "done")
	found.Set("done_at", clock.ISO(at))
	found.Pop("run_id")
	record := Record(s)
	checks := Checks(s)
	done := 0
	for _, c := range checks {
		if c.Str("status") == "done" {
			done++
		}
	}
	kept := []any{}
	for _, c := range checks {
		if c.Str("status") == "done" && done > Kept {
			done--
			continue // the oldest finished ones go first
		}
		kept = append(kept, c)
	}
	record.Set("checks", kept)
}
