package config

// `usage_history` (#2749): whether the status loop records every subscription's usage over time
// on the identity's usage branch (`_bot-usage`, `internal/usage`): each run's readings and the
// run itself, handed on by deliver, and the loop's own ticks, sessions and weeks. The night bot's
// config turns it on; Squishy's leaves it off, and its identity names no usage branch. A config
// that leaves it out records nothing.
func init() {
	optional.Set("usage_history", false)
}

// RecordsUsage is: this bot keeps a usage history (`usage_history` on, and its identity names a
// usage branch).
func (x *Config) RecordsUsage() bool { return x.UsageHistory && Identity.UsageBranch != "" }
