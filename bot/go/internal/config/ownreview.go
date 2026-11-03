package config

// The own review run (#2748, the owner's decision of 2026-11-02).
//
// `own_review_run`: a run that changes code (a build, a revision a person asked for, an auditor's
// fix, a CI fix) builds and runs the repository's checks but reviews nothing itself: its deliver
// opens or pushes the pull request, so CI starts, and hands its lane to a second run on the same
// subscription, its own review run, which reviews the change in fresh adversarial sessions, fixes
// what they find and approves it (`plan`'s own review, `work.Worker.OwnReview`,
// `deliver.Deliverer.OwnReview`). A fullsend part and a conflict-only carry keep their rules. A
// config that leaves it out, or sets it false (Squishy's), builds, checks and reviews in one run,
// as before.
//
// `own_review_reserve_minutes`: how long the lane stays reserved for that own review run once its
// build has delivered: no other item takes the lane meanwhile. Past it the lane is free again and
// the own review waits for its subscription as owned work does (`hold_minutes`), or is handed off
// as a final audit.
func init() {
	optional.Set("own_review_run", false)
	optional.Set("own_review_reserve_minutes", 30)
}

// LabelOwnReview is a bot pull request reserved for, waiting for or in its own review run (the
// live label of a review run, `bot:reviewing`): its builder's lane reviews it next.
var LabelOwnReview = LabelReviewing
