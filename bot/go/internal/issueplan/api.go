// Package issueplan is bot/harness/issueplan.py.
//
// An item's plan, kept in its issue's description (the Needs plan stage).
//
// A strong model plans every item before it is built (`plan.py`), and the plan goes into a marked
// **Plan** section at the end of the issue's description, so a person sees it where the task is and
// can correct it there: the builder starts from that section, whatever it says by then. The section
// carries no bot marker (`config.MARKER`), which would make the whole description read as the bot's.
package issueplan

// START = "<!-- jackioh-bot:plan -->"
var Start string = "<!-- jackioh-bot:plan -->"

// END = "<!-- /jackioh-bot:plan -->"
var End string = "<!-- /jackioh-bot:plan -->"

// BODY_LIMIT = 65_000
var BodyLimit int = 65000

// PLAN_WORDS = 2_500
var PlanWords int = 2500

// PLAN_CHARS = 30_000
var PlanChars int = 30000

// The description with its Plan section taken out.

func WithoutPlan(body string) string {
	panic("unported: issueplan.without_plan")
}

// The Plan section's text (a person's edits included), without its heading, or "".

func PlanOf(body string) string {
	panic("unported: issueplan.plan_of")
}

// The description with `plan` as its Plan section, in place of any earlier one.

func WithPlan(body string, plan string, who string, link string /* = "" */) string {
	panic("unported: issueplan.with_plan")
}
