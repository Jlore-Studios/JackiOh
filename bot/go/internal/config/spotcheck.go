package config

// `spot_check_every` (#2748, section 5): the after-merge spot-check. Every Nth medium or hard bot
// pull request that merges (counted in the state file, never drawn at random; no fullsend
// reconcile or follow-up) gets one review run by a strong model on a subscription other than its
// builder's, which blocks nothing and reverts nothing; its findings open one follow-up issue
// (`internal/spotcheck`). A config that leaves it out, or sets it to 0 (Squishy's), samples none.
func init() {
	optional.Set("spot_check_every", 0)
}

// SpotChecks is: this bot spot-checks merged changes at all (`spot_check_every` above 0).
func (x *Config) SpotChecks() bool { return x.SpotCheckEvery > 0 }
