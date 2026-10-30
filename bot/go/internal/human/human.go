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

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/easy"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/queue"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/triage"
)

// LetsGo are the labels that say the bot has an item, which come off when it hands the item
// over: the queue's state labels (`bot:blocked` among them), the Needs plan stage and the modes.
// What says what a thread is (`bot:pr`, `bot:pr-open`, `bot:planned`) stays.
var LetsGo = letsGo()

func letsGo() map[string]bool {
	out := map[string]bool{config.LabelNeedsPlan: true}
	for _, label := range queue.StateLabels {
		out[label] = true
	}
	for _, label := range config.ModeLabels.Keys() {
		out[label] = true
	}
	return out
}

// said is _SAID, `(?im)^[\s>*_]*human required[*_]*\s*:[*_]*\s*(.*)$`: a planner's own word that
// only a person can do the task: `**Human required:** <what>`.
var said = regexp.MustCompile(`(?im)^[` + py.SpaceClass + `>*_]*human required[*_]*` + py.ReS + `*:[*_]*` +
	py.ReS + `*(.*)$`)

// goal is _GOAL, `(?im)^\s*(?:#+\s*)?(?:\d+\.\s*)?\**\s*goal\b`: the plan's **Goal** heading: the
// planner's line goes before it (`bot/prompts/plan.md`). Only its start is read, so its `\b`
// (Unicode in Python, which RE2 has not) is written as the end of the text or a character after
// "goal" that is not a word character.
var goal = regexp.MustCompile(`(?im)^` + py.ReS + `*(?:#+` + py.ReS + `*)?(?:\p{Nd}+\.` + py.ReS +
	`*)?\**` + py.ReS + `*goal(?:$|` + py.ReNotW + `)`)

// headLines is how many lines open a plan with no **Goal** heading, where the planner's line may
// stand.
const headLines = 5

// nobody is what a planner writes on the line when no person is needed after all.
var nobody = map[string]bool{"no": true, "none": true, "n/a": true, "na": true, "nothing": true,
	"false": true, "not needed": true, "not required": true, "-": true}

// PlanNeeds is what a person must do before the bot can build any of `plan`, or "" when the bot
// can.
//
// The planner's `**Human required:**` line before **Goal** says it in its own words; a plan
// whose **Files to touch** table lists only paths in `forbidden` says it by itself. A plan that
// needs a person for one part only plans the rest and names that part under **Open
// questions**, and is built.
func PlanNeeds(plan string, forbidden []string) string {
	text := plan
	var head string
	if at := goal.FindStringIndex(text); at != nil {
		head = text[:at[0]]
	} else {
		lines := py.Splitlines(text)
		head = strings.Join(lines[:min(len(lines), headLines)], "\n")
	}
	found := said.FindStringSubmatch(head)
	words := ""
	if found != nil {
		words = strings.Join(strings.Fields(found[1]), " ")
	}
	if found != nil && !nobody[strings.ToLower(strings.Trim(words, " .*_"))] {
		if words == "" {
			return "Its planner says only a person can do it."
		}
		return words
	}
	paths := easy.PlanPaths(text)
	all := len(paths) > 0
	for _, path := range paths {
		all = all && git.Matches(path, forbidden)
	}
	if all {
		listed := []string{}
		for _, path := range paths[:min(len(paths), 8)] {
			listed = append(listed, "`"+path+"`")
		}
		return fmt.Sprintf("Every file its plan touches (%s) is in a path I may not change, such as ",
			strings.Join(listed, ", ")) +
			"`bot/`, `.harness/` and `.github/`, so a person has to make the change, from " +
			"the plan in the description."
	}
	return ""
}

// People is the people a `human required` item is handed to, as mentions.
func People() string {
	mentions := []string{}
	for _, login := range triage.Humans {
		mentions = append(mentions, "@"+login)
	}
	return strings.Join(mentions, " ")
}

// Ask is the comment that hands an item to a person: it names them, says what they must do, and
// how to give the item back. `what` is what the bot cannot do; `kept` notes work it kept;
// `again` is how to queue it again (a build, by default).
func Ask(question string, botLogin string, what string /* = "build this" */, kept string /* = "" */, again string /* = "" */) string {
	if again == "" {
		again = fmt.Sprintf("`%s build` (or `@%s <notes>`) queues it", config.Slash, botLogin)
	}
	q := strings.TrimSpace(question)
	if q == "" {
		q = "It needs something only a person can do."
	}
	return fmt.Sprintf("%s this needs a person: I cannot %s myself.%s\n\n", People(), what, kept) +
		q + "\n\n" +
		"So it is yours now: I took my labels off, unassigned myself, assigned you and " +
		fmt.Sprintf("labelled it `%s`, and I do not touch it while that label is on. ", config.LabelHumanRequired) +
		"If the rest is mine once a person has done their part, take the label off and " +
		fmt.Sprintf("%s; if you build it all yourselves, close it.", again)
}
