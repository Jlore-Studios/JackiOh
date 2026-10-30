// Package commands is bot/harness/commands.py.
//
// Comment commands: `/harness <verb> [args]`, `/harness-<verb>` and `@<bot> <verb or request>`.
//
// The slash is the running bot's (`config.SLASH`): `/harness` for the night bot, `/squishy` for
// Squishy (#60), whose modes add `oneshot` and `split`. Both have `fullsend` (#505).
//
// A command is a line of its own. Lines inside fenced code blocks and quoted lines (`> ...`) are
// never commands, so quoting the bot back at it cannot re-run anything. At most `MAX_COMMANDS` are
// read from one comment, top to bottom.
//
// Both prefixes read the rest of the line the same way: a verb (or an alias) runs; a single word
// that is a near miss of a verb runs nothing and gets a "did you mean"; anything else is a request:
// on an issue it asks for a build, on a pull request for a revision, with those words as the notes.
// The one difference is plain English: after `@<bot>`, a control verb followed by words that do not
// fit it is a request ("@bot start with option A"), unless a colon marks the words as the verb's
// own ("@bot halt: away this week").
package commands

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// VERBS: tuple[str, ...] = ("build", "revise", "review", "rebuild", "stop", "status", "help", "halt", "start", "suggest", "run", "suspend", "oneshot", "split", "f
var Verbs = []string{"build", "revise", "review", "rebuild", "stop", "status", "help", "halt", "start", "suggest", "run", "suspend", "oneshot", "split", "fullsend"}

// ALIASES: dict[str, str] = { "work": "build", "fix": "revise", "update": "revise", "resume": "start", "unhalt": "start", "go": "run", "suggestions": "suggest", }
var Aliases = map[string]string{"work": "build", "fix": "revise", "update": "revise", "resume": "start", "unhalt": "start", "go": "run", "suggestions": "suggest"}

// LEVELS: dict[str, int] = { "status": 1, "help": 1, "typo": 1, "build": 2, "oneshot": 2, "split": 2, "fullsend": 2, "revise": 2, "review": 2, "rebuild": 2, "stop
var Levels *pyjson.Object // unported

// FORCE_FLAG = "--force"
var ForceFlag string = "--force"

// FORCE_LEVEL = 3
var ForceLevel int = 3

// MAX_COMMANDS = 10
var MaxCommands int = 10

// Command is commands.Command.

type Command struct {
	Verb  string
	Args  string
	Force bool
	Line  string
	Meant string // default: ""
}

func (x *Command) Level() int {
	panic("unported: commands.Command.level")
}

// The verb or alias a request was probably meant to be, or None.
//
// Only a request of one word (an `#n` aside) counts: "stauts" is a typo, but "do the thing"
// is plain words even though "do" is one letter from "go". A long word may be two edits off,
// a short one only one.

func Misspelt(args string) string {
	panic("unported: commands.misspelt")
}

// The lines of `body` that may carry a command: outside fences, not quoted.

func CommandLines(body string) []string {
	panic("unported: commands.command_lines")
}

// The command in what follows a prefix. `plain_words` is the `@bot` form.

func Read(rest string, line string, plainWords bool) *Command {
	panic("unported: commands._read")
}

// Every command in a comment, in order, at most `MAX_COMMANDS`.

func Parse(body string, botLogin string) []*Command {
	panic("unported: commands.parse")
}

// VOTES: tuple[str, ...] = ("approve", "reject")
var Votes = []string{"approve", "reject"}

// The first vote in a comment, `(word, line)`, or `("", "")`. Quoted and fenced lines never
// vote, so quoting a vote back cannot cast it again.

func Vote(body string, botLogin string) (string, string) {
	panic("unported: commands.vote")
}

// True when the bot's handle, or `/harness`, appears anywhere outside quotes and code, even
// mid-sentence: a request that did not parse is answered with a hint, never ignored.

func NamesTheBot(body string, botLogin string) bool {
	panic("unported: commands.names_the_bot")
}

// HELP_ROWS: tuple[tuple[str, str], ...] = ( ("build", "| `build [notes]` | queue this issue for the bot | issue | 2 |"), ("oneshot", "| `oneshot [notes]` | build
var HelpRows [][2]string // unported

// Whether this bot has the verb: the modes only where they are, suggestions only where
// they are made.

func Offered(verb string) bool {
	panic("unported: commands.offered")
}

// FORCED = f"{', '.join(_FORCED[:-1])} or {_FORCED[-1]}"
var Forced any // unported

// HELP = ("""**Commands.** Write one per line, starting with `{slash}` or `@{bot}`; the two work the same way. | Verb | What it does | Where | Level | |---|---|--
var Help any // unported

// VERB_HELP: dict[str, tuple[str, str, str]] = { "build": ("build [notes]", "Queue this issue for the bot; your notes join the request. On a " "pull request it is
var VerbHelp *pyjson.Object // unported

// VOTE_HELP = ("**`approve`** or **`reject`** (level 2, maintainer), alone on a line of a comment on " "one of my open suggestions (`{prefix}suggestion`), bare or
var VoteHelp string = "**`approve`** or **`reject`** (level 2, maintainer), alone on a line of a comment on one of my open suggestions (`{prefix}suggestion`), bare or after `{slash}` or `@{bot}`. `approve` queues it to build, as `{prefix}build` does; `reject` closes it unbuilt, which makes room for the next survey. Anywhere else the words are only words.\n\nFor example: `approve`. "

// POINTER = "`{slash} help` (or `@{bot} help`) lists the commands."
var Pointer string = "`{slash} help` (or `@{bot} help`) lists the commands."

// The line that says where the commands are listed.

func Pointer_(bot string) string {
	panic("unported: commands.pointer")
}

// The reply to `help`, or to `help <verb>`.

func HelpText(bot string, topic string /* = "" */) string {
	panic("unported: commands.help_text")
}
