package config

// The bounded hold (#2748, section 1): a queued item whose owner, the subscription that planned
// it or took it up last, cannot start now only because of its 5-hour window, and can start again
// within `hold_minutes`, waits for that owner (`bot:waiting`, `plan.Owning`) instead of going to
// another subscription. `hold_exempt` names the subscriptions an item never waits for (Muse,
// whose 5-hour headroom rule does not apply): one of those that cannot take its item hands it off
// at once. A config that leaves `hold_minutes` out, or sets it to 0 (Squishy's), holds nothing.
func init() {
	optional.Set("hold_minutes", 0)
	optional.Set("hold_exempt", []any{})
}

// Holds is: this bot holds items for their owners at all (`hold_minutes` above 0).
func (x *Config) Holds() bool { return x.HoldMinutes > 0 }

// HoldsFor is: an item `providerID` owns may wait for it (`hold_exempt` does not name it).
func (x *Config) HoldsFor(providerID string) bool {
	if !x.Holds() {
		return false
	}
	for _, id := range x.HoldExempt {
		if id == providerID {
			return false
		}
	}
	return true
}
