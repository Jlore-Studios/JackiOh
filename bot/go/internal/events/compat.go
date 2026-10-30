package events

// Private helpers this package needs from Python itself: `int()` and `d[key]` with their
// exceptions, `type(exc).__name__`, and the running bot's constants from `config`, read through
// `any` so this package compiles whether `config` declares them as `any` or as their own types.

import (
	"fmt"
	"strconv"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
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

// act is `ctx.act`: the client for Actions calls.
func act(ctx *hctx.Context) gh.Client { return any(ctx.Act()).(gh.Client) }

// intersects is `bool(a & b)` for two sets of labels.
func intersects(names map[string]bool, others ...string) bool {
	for _, o := range others {
		if names[o] {
			return true
		}
	}
	return false
}

func contains(xs []string, s string) bool {
	for _, x := range xs {
		if x == s {
			return true
		}
	}
	return false
}

// containsAny is `v in (…)` for a value that may not be a string.
func containsAny(xs []string, v any) bool {
	s, ok := v.(string)
	return ok && contains(xs, s)
}

func cfgStr(v any) string {
	switch x := v.(type) {
	case nil:
		return ""
	case string:
		return x
	case fmt.Stringer:
		return x.String()
	}
	panic(fmt.Sprintf("events: config constant of type %T, not a string", v))
}

func cfgStrings(v any) []string {
	switch x := v.(type) {
	case nil:
		return nil
	case []string:
		return x
	case []any:
		return pyjson.Strings(x)
	}
	panic(fmt.Sprintf("events: config constant of type %T, not a list of strings", v))
}

// slash is config.SLASH.
func slash() string { return cfgStr(any(config.Slash)) }

// marker is config.MARKER.
func marker() string { return cfgStr(any(config.Marker)) }

// modes is config.MODES.
func modes() []string { return cfgStrings(any(config.Modes)) }

// modeLabel is `MODE_LABELS.get(name)`, and whether `name in MODE_LABELS`.
func modeLabel(name string) (string, bool) {
	switch x := any(config.ModeLabels).(type) {
	case nil:
		return "", false
	case map[string]string:
		m, ok := x[name]
		return m, ok
	case *pyjson.Object:
		v, ok := x.Lookup(name)
		return pyjson.Str(v), ok
	}
	panic(fmt.Sprintf("events: config.MODE_LABELS of type %T", config.ModeLabels))
}

// others is config.OTHERS.
func others() []*identity.Other {
	switch x := any(config.Others).(type) {
	case nil:
		return nil
	case []*identity.Other:
		return x
	case []identity.Other:
		out := make([]*identity.Other, len(x))
		for i := range x {
			out[i] = &x[i]
		}
		return out
	}
	panic(fmt.Sprintf("events: config.OTHERS of type %T", config.Others))
}

// ident is config.IDENTITY.
func ident() *identity.Identity {
	switch x := any(config.Identity).(type) {
	case nil:
		return &identity.Identity{}
	case *identity.Identity:
		return x
	case identity.Identity:
		return &x
	}
	panic(fmt.Sprintf("events: config.IDENTITY of type %T", config.Identity))
}

// levelName is `LEVEL_NAMES.get(level, str(level))`.
func levelName(level int) string {
	switch x := any(trust.LevelNames).(type) {
	case map[int]string:
		if name, ok := x[level]; ok {
			return name
		}
	case *pyjson.Object:
		if v, ok := x.Lookup(strconv.Itoa(level)); ok {
			return py.Str(v)
		}
	case nil:
	default:
		panic(fmt.Sprintf("events: trust.LEVEL_NAMES of type %T", trust.LevelNames))
	}
	return strconv.Itoa(level)
}

// The labels the queue uses (config.LABEL_*).
func labelBlocked() string    { return cfgStr(any(config.LabelBlocked)) }
func labelBuild() string      { return cfgStr(any(config.LabelBuild)) }
func labelPR() string         { return cfgStr(any(config.LabelPR)) }
func labelPROpen() string     { return cfgStr(any(config.LabelPROpen)) }
func labelRevise() string     { return cfgStr(any(config.LabelRevise)) }
func labelSuggestion() string { return cfgStr(any(config.LabelSuggestion)) }
func labelTree() string       { return cfgStr(any(config.LabelTree)) }
func labelWorking() string    { return cfgStr(any(config.LabelWorking)) }
func labelReady() string      { return cfgStr(any(config.LabelReady)) }
func labelPersonMerges() string {
	return cfgStr(any(config.LabelPersonMerges))
}
