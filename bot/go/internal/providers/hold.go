package providers

import (
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// RefusalMatch is REFUSAL_MATCH: how close a refusal's end (`refused_until`) must come to a
// window's reset in the last reading to count as that window's: a CLI names its reset to the
// minute or the hour ("resets 4am"), the reading to the second.
var RefusalMatch = 10 * time.Minute

// FiveHourWait is when `provider` can start a build, a revision or a plan again, if its 5-hour
// window alone keeps it from starting one now (#2748, section 1: the hold): its 5-hour reading at
// the cap in force or within `start_headroom` of it, or a refusal on that window, and nothing
// else: not its hours, a suspension, its login or a back-off (`Availability`), not its weekly
// window at its cap, nor a refusal on it (`WeeklyRefusal`), nor a minute budget. The time is
// `FreesAt`'s: its 5-hour reset, the start of its session's last hour where `last_hour` lifts the
// cap then, the end of the refusal, or its hours opening onto a looser cap, whichever comes first
// and lets it start. The zero time, and "", when anything else keeps it, or nothing does; else
// the time and the reason (its `Refusal`).
func FiveHourWait(provider *Provider, state *pyjson.Object, at time.Time, zoneName string, secrets *Secrets) (time.Time, string) {
	reason := Availability(provider, state, at, zoneName, secrets, false, true)
	entry := PeekRecord(state, provider.ID)
	if reason == "" || reason != Refusal(provider, entry, at, zoneName, true, time.Time{}) {
		return time.Time{}, "" // free, or held by something no reset lifts
	}
	if WeeklyRefusal(provider, entry, at) {
		return time.Time{}, ""
	}
	// Without its 5-hour window (its reading, and a refusal on it), nothing may hold it.
	rest := entry.Copy()
	rest.Delete("refused_until")
	usage := CurrentUsage(provider, entry).Copy()
	usage.Delete("five_hour")
	rest.Set("usage", usage)
	if Refusal(provider, rest, at, zoneName, true, time.Time{}) != "" {
		return time.Time{}, ""
	}
	frees := FreesAt(provider, entry, at, zoneName)
	if frees.IsZero() {
		return time.Time{}, ""
	}
	return frees, reason
}

// WeeklyRefusal is whether the refusal still in force on `provider` (`refused_until`) is its
// week's, not its 5-hour session's: it ends later than any 5-hour session can, or within
// `RefusalMatch` of its week's reset in the last reading, or (ending at neither reset) while that
// reading had its week nearly full (`NearFull`) and its session not. False with no refusal in
// force.
func WeeklyRefusal(provider *Provider, entry *pyjson.Object, at time.Time) bool {
	until := clock.ParseISOValue(entry.Get("refused_until"))
	if until.IsZero() || !until.After(at) {
		return false
	}
	if until.Sub(at) > Windows["five_hour"] {
		return true
	}
	usage := CurrentUsage(provider, entry)
	near := func(window string) (bool, bool) {
		reading, ok := mapping(usage.Get(window))
		if !ok {
			return false, false
		}
		resets := clock.ParseISOValue(reading.Get("resets_at"))
		matches := !resets.IsZero() && absDuration(resets.Sub(until)) <= RefusalMatch
		used, isNumber := py.Number(reading.Get("utilization"))
		return matches, isNumber && used >= NearFull
	}
	sessionEnds, sessionFull := near("five_hour")
	weekEnds, weekFull := near("seven_day")
	switch {
	case sessionEnds:
		return false
	case weekEnds:
		return true
	}
	return weekFull && !sessionFull
}

func absDuration(d time.Duration) time.Duration {
	if d < 0 {
		return -d
	}
	return d
}
