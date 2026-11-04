package config

// A question goes ahead on its default (#2986, section 2; the owner's decision of 2026-11-04). A
// builder or reviser that stops to ask names the one option that stays inside the issue's own
// limits, its default; with no trusted answer within `decision_wait_hours`, the subscription and
// model that asked take it up again in a fresh session and build the default (`plan.SyncDecisions`).
// 0, and a config that leaves it out, never goes ahead: every question waits for a person.
func init() {
	optional.Set("decision_wait_hours", 0)
}

// LabelNeedsDecision is a stop only a person can unblock (#2986, section 3): a question, with or
// without a default, or a hand-over (`human required`). It is shared by both bots, as `ready for
// merge` is, and sits beside `bot:blocked` (or `human required`), which keep their meanings. It
// comes off when a trusted person answers (the item is queued again, `queue.SetStateLabel`), when the
// bot goes ahead on the default, and when the item closes (`plan.SyncDecisions`).
var LabelNeedsDecision = "needs decision"

// DecisionsGoAhead is: this bot goes ahead on a question's default at all (`decision_wait_hours`
// above 0).
func (x *Config) DecisionsGoAhead() bool { return x.DecisionWaitHours > 0 }
