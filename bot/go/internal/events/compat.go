package events

// The helpers events.py's calls are written with: a user's trust level, and the set operations
// it writes with `in` and `&`.

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
)

// userID is a user's `id` as trust.level takes it: None when absent.
func userID(v any) *int {
	if v == nil {
		return nil
	}
	n := py.MustInt(v)
	return &n
}

// level is `ctx.trust.level(login, user_id, association)`.
func trustLevel(t *trust.Trust, login string, id any, association any) int {
	assoc := ""
	if association != nil {
		assoc = py.Str(association)
	}
	return t.Level(login, userID(id), assoc)
}

// intersects is `bool(a & b)` for two sets of labels.
func intersects(names map[string]bool, others ...string) bool {
	for _, o := range others {
		if names[o] {
			return true
		}
	}
	return false
}

// modeLabel is `MODE_LABELS.get(name)`, and whether `name in MODE_LABELS`.
func modeLabel(name string) (string, bool) {
	v, ok := config.ModeLabels.Lookup(name)
	return pyjson.Str(v), ok
}
