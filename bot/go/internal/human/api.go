// Package human is bot/harness/human.py.
//
// Work only a person can do (#793): the `human required` label.
//
// The bot may not change `bot/`, `.harness/`, `.squishy/` or `.github/` (the path guard puts such a
// change back, and deliver refuses to push one), holds no secret it was not given, and has no
// dashboard: GitHub's settings and secrets, Vercel, Render, Supabase and Cloudflare are a person's.
// No run can finish an item that needs one of those, and asking the same question run after run
// spends a run each time (#760 spent two). So the bot hands it over: its own labels come off
// (`LETS_GO`: `bot:blocked`, the queue and the stages), it unassigns itself and assigns the people
// (`triage.HUMANS`, as `human` does), labels it `human required` and names them in a comment that
// says what a person must do (`ask`). The item is then people's, as one labelled `human` is
// (`queue.is_human`): an item is `human` or `human required` (people's) or carries the bot's
// labels (the bot's), never both. A person who did only their part takes the label off and queues
// the rest; one who builds it all closes it.
//
// What says an item needs a person:
//
// - its plan's **Files to touch** table lists only paths the bot may not change (`plan_needs`);
// - its planner says so, in a `**Human required:**` line before the plan's **Goal** (`plan_needs`);
// - its builder or reviser stopped with `"human_required": true` (`verdicts.build_report`);
// - the path guard put the whole of a build's change back (`work.Worker._only_forbidden`).
package human

// LETS_GO: frozenset[str] = frozenset({*STATE_LABELS, LABEL_NEEDS_PLAN, *MODE_LABELS})
var LetsGo map[string]bool // unported

// What a person must do before the bot can build any of `plan`, or "" when the bot can.
//
// The planner's `**Human required:**` line before **Goal** says it in its own words; a plan
// whose **Files to touch** table lists only paths in `forbidden` says it by itself. A plan that
// needs a person for one part only plans the rest and names that part under **Open
// questions**, and is built.

func PlanNeeds(plan string, forbidden []string) string {
	panic("unported: human.plan_needs")
}

// The people a `human required` item is handed to, as mentions.

func People() string {
	panic("unported: human.people")
}

// The comment that hands an item to a person: it names them, says what they must do, and
// how to give the item back. `what` is what the bot cannot do; `kept` notes work it kept;
// `again` is how to queue it again (a build, by default).

func Ask(question string, botLogin string, what string /* = "build this" */, kept string /* = "" */, again string /* = "" */) string {
	panic("unported: human.ask")
}
