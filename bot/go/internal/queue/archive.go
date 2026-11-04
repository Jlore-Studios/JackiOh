package queue

// The bot's work on an item it handed to a person, archived (#3009, section 2; the owner's decision
// of 2026-11-04). `human required` means the bot decided it cannot do the item itself, so it forgets
// it: its record's queue state, holds, turns, lane reservations, decision, owner and merge owner, CI
// fixes and kept session go (`ArchiveDrops`), and an `archived` entry stays (when, why, the branch,
// the pull request and its last head), on the item and on the thread it links (an issue's open pull
// request, a pull request's issue). While it is there no sweep, housekeeping, CI fix, conflict
// revision, merge-queue requeue, turn or spot-check takes either up (`ArchivedOf`, read by each).
// A person who wants the bot back takes `human required` off and queues it: the claim of that run
// drops the entries (`Unarchive`), and the item starts from its issue and its branch's history.

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// ArchivedKey is the record's field of an item the bot handed to a person.
var ArchivedKey = "archived"

// ArchiveDrops are what an archived item's records forget: the queue state, the holds, the turns
// and lane reservations, the decision, the merge owner, the CI fixes and the kept session. The owner
// is released apart (`Archive`), as a handoff releases it.
var ArchiveDrops = []string{
	// the queue state
	"kind", "queued_at", "forced", "failures", "source", "requested_by", "pending_request",
	"stop_requested", "verdict_rounds", "round_tier",
	// the holds
	"hold", "handoff_requested", "waiting_comment",
	// the turns and the lane reservations
	WaitingTurnKey, TurnKey, BuildNextKey, InLaneKey, OwnReviewKey,
	// the decision
	DecisionKey,
	// the merge owner
	"merge_owner", GaveUpKey,
	// the CI fixes
	"ci_fixes", "ci_fixed_head", "ci_run_id", "ci_sha", "ci_reruns", "ci_early", "ci_on_head",
	// the kept session
	"session",
	// the owner's
	"owner_since", "owner_why",
}

// ArchivedOf is the record's `archived` entry, or nil when the bot has not archived it.
func ArchivedOf(record *pyjson.Object) *pyjson.Object {
	found, ok := record.Get(ArchivedKey).(*pyjson.Object)
	if !ok || found == nil || !found.Truthy("at") {
		return nil
	}
	return found
}

// Archive forgets thread `number` in `s` (`ArchiveDrops`, and its owner released) and keeps `entry`
// as its `archived`; with `linked` (its open pull request, or a pull request's issue; 0 for none)
// the same for that thread, its entry naming `number` back.
func Archive(s *pyjson.Object, number int, linked int, entry *pyjson.Object) {
	forget := func(n int, kept *pyjson.Object) {
		record := stateItem(s, n)
		for _, key := range ArchiveDrops {
			record.Pop(key)
		}
		if record.Has("owner") || record.Has("planned_by_id") {
			record.Set("owner", "") // released, as a handoff releases it (`OwnerOf`)
		}
		record.Set(ArchivedKey, kept)
	}
	forget(number, entry)
	if linked != 0 && linked != number {
		back := entry.Copy()
		back.Set("item", number)
		forget(linked, back)
	}
}

// Unarchive drops thread `number`'s `archived` entry, and that of each thread it links (its pull
// request, its issue, the item it was archived with). Returns whether there was one.
func Unarchive(s *pyjson.Object, number int) bool {
	entry := ArchivedOf(recordOf(s, number))
	if entry == nil {
		return false
	}
	stateItem(s, number).Pop(ArchivedKey)
	for _, key := range []string{"pr", "issue", "item"} {
		if other := py.MustInt(py.Or(entry.Get(key), 0)); other != 0 && other != number {
			if ArchivedOf(recordOf(s, other)) != nil {
				stateItem(s, other).Pop(ArchivedKey)
			}
		}
	}
	return true
}
