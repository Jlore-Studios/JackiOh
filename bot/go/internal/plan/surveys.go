package plan

import (
	"slices"
	"strconv"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// The suggestion surveys running now (#4026). Several run at once, each on its own spare lane and
// its own area of the repository (`suggestions.areas`). Each one's record is
// `suggest.running[<slot>]` (its `provider`, `run_id`, `area`, `count`, `last_run` and
// `taken_asks`), and it holds lane -slot, so a survey's lane never meets an item's. A claim from
// before #4026, the one survey `suggest.provider` and `suggest.run_id` held, is lane 0 until it
// ends.

// Survey is one running survey: its lane and its record.
type Survey struct {
	Lane   int
	Record *pyjson.Object
}

// IsSurveyLane is whether lane `number` is a survey's rather than an item's.
func IsSurveyLane(number int) bool { return number <= 0 }

// Surveys is the surveys the state file says are running: lane 0's first, then by slot.
func Surveys(state *pyjson.Object) []Survey {
	suggest := state.Obj("suggest")
	var found []Survey
	if suggest.Truthy("provider") {
		found = append(found, Survey{0, suggest})
	}
	running := suggest.Obj("running")
	var slots []int
	for _, key := range running.Keys() {
		if slot, err := strconv.Atoi(key); err == nil && slot > 0 && running.Obj(key).Truthy("provider") {
			slots = append(slots, slot)
		}
	}
	slices.Sort(slots)
	for _, slot := range slots {
		found = append(found, Survey{-slot, running.Obj(strconv.Itoa(slot))})
	}
	return found
}

// SurveyRecord is the record of the survey on lane `number`: `suggest` itself for lane 0, and an
// empty record when no survey runs there.
func SurveyRecord(state *pyjson.Object, number int) *pyjson.Object {
	suggest := state.Obj("suggest")
	if number == 0 {
		if suggest.Len() > 0 {
			return suggest
		}
		return pyjson.New()
	}
	if record := suggest.Obj("running").Obj(strconv.Itoa(-number)); record != nil {
		return record
	}
	return pyjson.New()
}

// SurveyLaneOf is the lane of the survey `planned` claimed: its `slot`'s, or 0 for a plan from
// before #4026, which carries none.
func SurveyLaneOf(planned *pyjson.Object) int {
	return -planned.Int("slot")
}

// EndSurvey takes the survey on lane `number` off `s` (inside a state update) and returns its
// record, which still holds its asks.
func EndSurvey(s *pyjson.Object, number int) *pyjson.Object {
	suggest := pyjson.Obj(s.SetDefault("suggest", pyjson.New()))
	if number == 0 {
		record := pyjson.New("taken_asks", suggest.Get("taken_asks"))
		suggest.Update(pyjson.New("provider", nil, "run_id", nil, "taken_asks", []any{}))
		return record
	}
	running := suggest.Obj("running")
	record := running.Obj(strconv.Itoa(-number))
	running.Delete(strconv.Itoa(-number))
	if record == nil {
		return pyjson.New()
	}
	return record
}

// SurveyAsks is the asks (`/harness suggest` comments) a survey's record took when it was claimed.
func SurveyAsks(record *pyjson.Object) []string {
	var keys []string
	for _, k := range py.Each(record.Get("taken_asks")) {
		keys = append(keys, py.Str(k))
	}
	return keys
}

// reservedSuggestions is how many suggestions the running surveys may still open: each one's
// `count`. A survey from before #4026 was given all the room there was, so it keeps all of it.
func reservedSuggestions(cfg *config.Config, surveys []Survey) int {
	total := 0
	for _, s := range surveys {
		if s.Lane == 0 {
			total += cfg.SuggestionsMaxOpen
		} else {
			total += s.Record.Int("count")
		}
	}
	return total
}

// MaxSurveys is how many surveys may run at once: one an area, or one when there are none.
func MaxSurveys(cfg *config.Config) int {
	return max(1, len(cfg.SuggestionAreas))
}

// freeAreas is the areas no running survey holds, in turn: those after `suggest.last_area` first,
// so each gets surveyed in its turn.
func freeAreas(cfg *config.Config, state *pyjson.Object, surveys []Survey) []config.SuggestionArea {
	held := map[string]bool{}
	for _, s := range surveys {
		held[s.Record.Str("area")] = true
	}
	areas := cfg.SuggestionAreas
	start := 0
	last := state.Obj("suggest").Str("last_area")
	for i, a := range areas {
		if a.ID == last {
			start = i + 1
		}
	}
	var free []config.SuggestionArea
	for i := range areas {
		if a := areas[(start+i)%len(areas)]; !held[a.ID] {
			free = append(free, a)
		}
	}
	return free
}

// freeSlot is the lowest slot no running survey holds.
func freeSlot(surveys []Survey) int {
	slot := 1
	for slices.ContainsFunc(surveys, func(s Survey) bool { return s.Lane == -slot }) {
		slot++
	}
	return slot
}
