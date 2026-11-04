package config

// A question a person answers and the bot carries out (#2986, section 2; #3009, section 1; the
// owner's decisions of 2026-11-04). A builder or reviser that stops to ask explains the problem,
// numbers its options with their trade-offs and names the one it suggests, its default. Setting
// `needs decision` assigns `decision_assignees`; a trusted person's reply revises it in a fresh
// session on the lane that asked, and with no trusted answer within `decision_wait_hours` that lane
// goes ahead on the default (`plan.SyncDecisions`). 0, and a config that leaves it out, never goes
// ahead: every question waits for a person.
func init() {
	optional.Set("decision_wait_hours", 0)
	optional.Set("decision_assignees", []any{})
}

// LabelNeedsDecision is a question a person answers and the bot carries out (#2986, section 3;
// #3009, section 1). It is shared by both bots, as `ready for merge` is, and sits beside
// `bot:blocked`, which keeps its meaning. A hand-over (`human required`) never carries it: the bot
// decided it cannot do that item itself. It comes off, and the people the bot assigned with it are
// unassigned, when a trusted person answers (the item is queued again, `queue.SetStateLabel`), when
// the bot goes ahead on the default, and when the item closes (`plan.SyncDecisions`).
var LabelNeedsDecision = "needs decision"

// DecisionsGoAhead is: this bot goes ahead on a question's default at all (`decision_wait_hours`
// above 0).
func (x *Config) DecisionsGoAhead() bool { return x.DecisionWaitHours > 0 }
