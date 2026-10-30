// Package asks is bot/harness/asks.py.
//
// Reactions that follow a request through the queue, so a person sees where it has got to.
//
// A comment that asks for model work (a build, a revision, a request in plain words, a suggestion
// survey, or notes left while a run holds the thread) is recorded as an *ask* on the state record
// it waits on: the thread's item, or `state["suggest"]`. An ask is the key its claim uses, `c:<id>`
// for a conversation comment and `rc:<id>` for a line comment on a diff. A review cannot take a
// reaction, so a review's request is not tracked. Each stage reacts to the ask:
//
// | Reaction | Meaning | Left by |
// |---|---|---|
// | 👀 `eyes` | seen | events, before acting |
// | 👍 `+1` | a model will read it: queued, or noted for the next pass | events |
// | 🚀 `rocket` | answered; the sweep reads it as "handled" | events, after replying |
// | ❤️ `heart` | a run has it: a model is reading it now | plan, claiming the thread |
// | 🎉 `hooray` | done: the run that read it finished with an answer | deliver |
// | 😕 `confused` | it ended without an answer: blocked, stopped or given up on | deliver, stop |
//
// An ask waits in `asks` until a run claims the thread, which moves it to `taken_asks`. When the
// run ends, each taken ask gets 🎉 or 😕, or goes back to waiting if the item is queued again. An
// ask that arrives while a run holds the thread waits for the next pass. The helpers that change a
// record are pure, for use inside `StateStore.update`; `react` is called once the write is done.
package asks

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// SEEN = "eyes"
var Seen string = "eyes"

// QUEUED = "+1"
var Queued string = "+1"

// ANSWERED = "rocket"
var Answered string = "rocket"

// WORKING = "heart"
var Working string = "heart"

// DONE = "hooray"
var Done string = "hooray"

// NO_ANSWER = "confused"
var NoAnswer string = "confused"

// MAX_ASKS = 20
var MaxAsks int = 20

// The comment a request came from. `recorded` says whether anything queued it.

type Ask struct {
	Key      string
	Recorded bool // default: False
}

// key is asks.key.

func Key(commentID int, reviewComment bool /* = False */) string {
	panic("unported: asks.key")
}

// Record `ask` as waiting on `holder`.

func Note(holder *pyjson.Object, ask *Ask) {
	panic("unported: asks.note")
}

// add is asks.add.

func Add(holder *pyjson.Object, keys []string) {
	panic("unported: asks.add")
}

// A run has claimed the holder: its waiting asks become taken. Returns those that moved.

func Take(holder *pyjson.Object) []string {
	panic("unported: asks.take")
}

// The run ended without an answer and the item is queued again: its asks wait once more.

func GiveBack(holder *pyjson.Object) {
	panic("unported: asks.give_back")
}

// pop_taken is asks.pop_taken.

func PopTaken(holder *pyjson.Object) []string {
	panic("unported: asks.pop_taken")
}

// pop_waiting is asks.pop_waiting.

func PopWaiting(holder *pyjson.Object) []string {
	panic("unported: asks.pop_waiting")
}

// Leave `content` on every ask in `keys`. A reaction is a courtesy: `gh.react` swallows its
// own failures.

func React(gh gh.Client, keys []string, content string) {
	panic("unported: asks.react")
}
