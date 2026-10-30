package sweep

// Private helpers this package needs from Python itself (`int()`, `d[key]` and their exceptions,
// `type(exc).__name__`) and the running bot's constants from `config`, read through `any` so this
// package compiles whether `config` declares them as `any` or as their own types.

import (
	"fmt"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

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

// intersects is `bool(names & set(others))`.
func intersects(names map[string]bool, others []string) bool {
	for _, o := range others {
		if names[o] {
			return true
		}
	}
	return false
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
	panic(fmt.Sprintf("sweep: config constant of type %T, not a string", v))
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
	panic(fmt.Sprintf("sweep: config constant of type %T, not a list of strings", v))
}

// modes is config.MODES.
func modes() []string { return cfgStrings(any(config.Modes)) }

// retiredLabels is config.RETIRED_LABELS.
func retiredLabels() []string { return cfgStrings(any(config.RetiredLabels)) }

// nightWorkflow is config.NIGHT_WORKFLOW.
func nightWorkflow() string { return cfgStr(any(config.NightWorkflow)) }

// labelSpec is one of config.LABELS: name: (color, description).
type labelSpec struct{ name, color, description string }

// labels is `LABELS.items()`, in order.
func labels() []labelSpec {
	var out []labelSpec
	var o *pyjson.Object
	switch x := any(config.Labels).(type) {
	case nil:
		return nil
	case *pyjson.Object:
		o = x
	default:
		panic(fmt.Sprintf("sweep: config.LABELS of type %T", config.Labels))
	}
	o.Items(func(name string, v any) bool {
		var pair []string
		switch t := v.(type) {
		case []any:
			pair = pyjson.Strings(t)
		case []string:
			pair = t
		case [2]string:
			pair = t[:]
		}
		if len(pair) != 2 {
			panic(fmt.Sprintf("sweep: config.LABELS[%q] of type %T", name, v))
		}
		out = append(out, labelSpec{name, pair[0], pair[1]})
		return true
	})
	return out
}

// The labels the queue uses (config.LABEL_*).
func labelBlocked() string { return cfgStr(any(config.LabelBlocked)) }
func labelBuild() string   { return cfgStr(any(config.LabelBuild)) }
func labelPlanned() string { return cfgStr(any(config.LabelPlanned)) }
func labelPR() string      { return cfgStr(any(config.LabelPR)) }
func labelPROpen() string  { return cfgStr(any(config.LabelPROpen)) }
func labelRevise() string  { return cfgStr(any(config.LabelRevise)) }
func labelTree() string    { return cfgStr(any(config.LabelTree)) }
func labelWorking() string { return cfgStr(any(config.LabelWorking)) }
