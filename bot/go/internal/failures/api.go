// Package failures is bot/harness/failures.py.
//
// Why a run's rounds failed, written for the person who reviews it.
//
// A build or revision that uses up every review round it is given (`max_review_cycles`) without an
// approval is labelled `bot:stuck`, and its comment says why, round by round: what the builder's
// session did, which checks were red, what the reviewer said, and which findings kept coming back.
// A run that stopped sooner (an unreadable review, a refusal) says its real round count and reason.
package failures

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// TOP_FINDINGS = 5
var TopFindings int = 5

// CELL_CHARS = 140
var CellChars int = 140

// _cell is failures._cell.

func Cell(text any, limit int /* = CELL_CHARS */) string {
	panic("unported: failures._cell")
}

// It used every round it was given.

func Stuck(cycles []*pyjson.Object, limit int) bool {
	panic("unported: failures.stuck")
}

// Markdown: why the rounds failed, a summary and then round by round.

func Why(cycles []*pyjson.Object, limit int, reason string /* = "" */) string {
	panic("unported: failures.why")
}
