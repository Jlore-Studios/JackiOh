// Package prompts is bot/harness/prompts.py.
//
// Prompt templates from `bot/prompts/<name>.md`, filled with `string.Template`.
//
// Text that came from GitHub or from a tool is wrapped by `data()` in a fence longer than any
// backtick run inside it, under a heading that says it is data, not instructions.
package prompts

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// NAMES = ("system", "plan", "build", "fix", "revise", "review", "suggest", "oneshot", "split", "reconcile")
var Names = []string{"system", "plan", "build", "fix", "revise", "review", "suggest", "oneshot", "split", "reconcile"}

// load is prompts.load.

func Load(name string, directory string /* = PROMPTS_DIR */) any {
	panic("unported: prompts.load")
}

// The template with every placeholder filled. A missing value raises KeyError.

func Render(name string, directory string /* = PROMPTS_DIR */, values *pyjson.Object) string {
	panic("unported: prompts.render")
}

// placeholders is prompts.placeholders.

func Placeholders(name string, directory string /* = PROMPTS_DIR */) map[string]bool {
	panic("unported: prompts.placeholders")
}

// `text` fenced as data. The fence is longer than any backtick run inside.

func Data(text string, label string) string {
	panic("unported: prompts.data")
}
