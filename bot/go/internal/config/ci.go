package config

// The CI workflow's jobs that count as required checks (#2948), for the CI reader of an own review
// run (`work.Worker.ciRed`) and the status loop's early stop of a bot pull request's red CI
// (`events.StopRedCI`).
//
// Branch protection waits on the five names of `required_checks` (ci.yml's header, infra/github.tf).
// Four of them are summary jobs that need their pieces and fail unless every piece passed, and the
// pieces are named after them (`rust (test engine)`, `web (unit 1/4)`, `e2e smoke (1/2)`,
// `db (sql)`), so a failed piece is a red required check before its summary has run. A summary
// never runs while a piece still runs, and it is skipped when its run is cancelled, so a reader of
// a run the bot stopped early finds the failed pieces red, not the summary. `CINeeded` names the
// jobs a required check needs that carry no check's name. tests/workflows_test.go holds this to
// ci.yml: every job a required check needs, on a pull request, is a required job here.

import "strings"

// CIRed is the conclusions of a CI job that make a required check red. A job cancelled says
// nothing of the change (a newer push replaced its run, or the bot stopped it, `events.StopRedCI`).
var CIRed = []string{"failure", "timed_out"}

// CINeeded are the CI jobs a required check needs that are named after none (each a name, or the
// start of one before ` (`): `changes`, which every check needs, and `e2e component`, which
// `e2e smoke` needs in a run of every spec.
var CINeeded = []string{"what the change touches", "e2e component"}

// RequiredJob is whether a CI job named `name` counts as a required check of `required`: the
// check's own job (`rust`), one of the jobs it gathers (`rust (test engine)`), or a job of
// `CINeeded`.
func RequiredJob(name string, required []string) bool {
	for _, check := range append(append([]string{}, required...), CINeeded...) {
		if name == check || strings.HasPrefix(name, check+" (") {
			return true
		}
	}
	return false
}
