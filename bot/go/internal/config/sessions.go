package config

// `resume_sessions` (#2748): whether a run on a Claude account keeps its own Claude Code session
// for the next run of its item on the same account, which resumes it (`internal/sessions`). A
// config that leaves it out keeps none, as every config did before it.
func init() {
	optional.Set("resume_sessions", false)
}
