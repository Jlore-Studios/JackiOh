package journal

import (
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// SessionLine is a run's line on its own Claude session (the result's `session`, #2748): whether
// its call went on in the session the item's last run on this subscription kept, or why it ran
// fresh with the handoff; what the resumed call (or the attempt) took; and what became of this
// run's own session.
func SessionLine(session *pyjson.Object) string {
	from := session.Obj("from")
	var line string
	if session.Truthy("resumed") {
		line = "Session: resumed `" + py.SliceTo(from.Str("id"), 8) + "`"
		if from.Truthy("provider") {
			line += ", kept by the run on `" + from.Str("provider") + "`"
			if at := Clock(from.Get("at")); at != "" {
				line += " at " + at
			}
		}
	} else {
		line = "Session: fresh"
		if session.Truthy("tried") {
			line += ", with the handoff, since the kept one could not be resumed"
		}
		line += ": " + Cell(py.OrStr(session.Get("fallback"), "none was kept for this item"))
	}
	if took := Took(session.Obj("call")); took != "" {
		if session.Truthy("resumed") {
			line += "; that call took " + took
		} else {
			line += "; the attempt took " + took
		}
	}
	switch {
	case session.Truthy("delivered"):
		line += ". This run's: " + Cell(session.Get("delivered"))
	case session.Obj("kept").Len() > 0:
		kept := session.Obj("kept")
		line += ". This run's own session `" + py.SliceTo(kept.Str("id"), 8) + "` was sealed (" +
			py.Thousands(kept.Int("bytes")) + " bytes)"
	case session.Truthy("not_kept"):
		line += ". This run's own session was not kept: " + Cell(session.Get("not_kept"))
	}
	return strings.TrimRight(line, ".") + "."
}
