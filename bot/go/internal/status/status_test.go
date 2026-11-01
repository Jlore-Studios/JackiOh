package status

import (
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

func TestARecordIsAnItemsOrTheSurveys(t *testing.T) {
	state := pyjson.New("items", pyjson.New("5", pyjson.New("action", "plan"), "6", nil),
		"suggest", pyjson.New("provider", "gpt"))
	if got := RecordOf(state, 5).Get("action"); got != "plan" {
		t.Errorf("item 5: %v", got)
	}
	if got := Record(state, 0).Get("provider"); got != "gpt" {
		t.Errorf("the survey: %v", got)
	}
	if Record(state, 7).Len() != 0 || Record(pyjson.New("items", pyjson.New()), 0).Len() != 0 {
		t.Error("a missing record reads as empty")
	}
}
