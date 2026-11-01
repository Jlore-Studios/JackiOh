package plan

// ---------------------------------------------------------------------- lanes and matching

import (
	"fmt"
	"slices"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Lanes is the runs that hold work now: item number (0 for a suggestion survey) -> provider id.
// `planning` are the items whose run is a planning run: those hold the planning lane
// (`Pool.plan_lanes`), not a lane of their provider's. GitHub's runners and the bot's machine
// have lanes of their own, so neither waits on the other: `limit` is GitHub's
// (`Pool.max_parallel`), `machine_limit` the machine's (`Pool.machine_parallel`), and `machine`
// names the subscriptions that run on the machine. Every run holds a runner where it runs, a
// planning run too, so `limit` bounds the hosted runners the bot's model jobs take (#1455) and
// `machine_limit` the machine's slots. `limit` is a pool the bots share, first come first
// served: `shared` is how many of its slots the other bots' runs hold now (`others_on_github`).
// `filling` are the items whose run fills another subscription's lane (`Provider.fill_in`):
// those hold no lane of their provider's own.
//
// The maps may be nil (the dataclass's empty defaults): every method only reads them, and
// `with_run` makes new ones.
type Lanes struct {
	Limit        int
	Held         map[int]string  // default: field(default_factory=dict)
	Planning     map[int]bool    // default: field(default_factory=set)
	PlanLimit    int             // default: 0
	MachineLimit int             // default: 0
	Machine      map[string]bool // default: frozenset()
	Shared       int             // default: 0
	Filling      map[int]bool    // default: field(default_factory=set)
}

// Busy is the set of providers holding a run.
func (x *Lanes) Busy() map[string]bool {
	busy := map[string]bool{}
	for _, held := range x.Held {
		busy[held] = true
	}
	return busy
}

// heldBy is `self.held.get(number) == provider_id`.
func (x *Lanes) heldBy(number int, providerID string) bool {
	held, ok := x.Held[number]
	return ok && held == providerID
}

// Count is how many runs this provider holds now, its planning runs aside.
func (x *Lanes) Count(providerID string) int {
	n := 0
	for number, held := range x.Held {
		if held == providerID && !x.Planning[number] {
			n++
		}
	}
	return n
}

// PlanningBy is how many planning runs this provider holds now.
func (x *Lanes) PlanningBy(providerID string) int {
	n := 0
	for number := range x.Planning {
		if x.heldBy(number, providerID) {
			n++
		}
	}
	return n
}

// FillingBy is how many runs this provider holds in other subscriptions' lanes (`fill_in`).
func (x *Lanes) FillingBy(providerID string) int {
	n := 0
	for number := range x.Filling {
		if x.heldBy(number, providerID) {
			n++
		}
	}
	return n
}

// AllBy is every run this provider holds now, its planning runs and fill-ins too.
func (x *Lanes) AllBy(providerID string) int {
	n := 0
	for _, held := range x.Held {
		if held == providerID {
			n++
		}
	}
	return n
}

// Used is how many of its own `lanes` it holds now: its runs, those in other subscriptions'
// lanes (`fill_in`) aside. A subscription the quiet check guards, or one with usage caps,
// counts its planning run too: the quiet check cannot tell a second run of the bot's from
// its owner, and two runs deciding from one reading go past a cap together (claude-2
// planned and revised at once from 0% and was refused at 100% 15 minutes later).
func (x *Lanes) Used(provider *providers.Provider) int {
	held := x.Count(provider.ID) - x.FillingBy(provider.ID)
	if provider.QuietCheck || pyjson.Truthy(provider.Limits.Stops()) {
		held += x.PlanningBy(provider.ID)
	}
	return held
}

// Full is: it holds as many runs as its `lanes` (`used`).
func (x *Lanes) Full(provider *providers.Provider) bool {
	return x.Used(provider) >= provider.Lanes
}

// OnMachine is how many of the held runs are on the bot's machine.
func (x *Lanes) OnMachine(pool *providers.Pool) int {
	n := 0
	for _, providerID := range x.Held {
		if pool.OnMachine(providerID) {
			n++
		}
	}
	return n
}

// Free is the lanes free on GitHub's runners. A planning run there takes one too (#1455): the
// organization runs 20 hosted jobs at once, CI among them, so `max_parallel` caps every
// model job the bots run on them, not only builds, and the other bots' runs take theirs
// out of the same pool (`shared`).
func (x *Lanes) Free() int {
	hosted := 0
	for _, providerID := range x.Held {
		if !x.Machine[providerID] {
			hosted++
		}
	}
	return max(0, x.Limit-x.Shared-hosted)
}

// MachineFree is the slots free on the machine; a planning run there takes one too
// (`machine_full`).
func (x *Lanes) MachineFree() int {
	there := 0
	for _, providerID := range x.Held {
		if x.Machine[providerID] {
			there++
		}
	}
	return max(0, x.MachineLimit-there)
}

// RoomFor is: a lane is free where `provider_id` runs: the machine, or GitHub's runners.
func (x *Lanes) RoomFor(providerID string) bool {
	if x.Machine[providerID] {
		return x.MachineFree() > 0
	}
	return x.Free() > 0
}

// AnyRoom is plan.Lanes.any_room.
func (x *Lanes) AnyRoom() bool {
	return x.Free() > 0 || x.MachineFree() > 0 || x.PlanFree() > 0
}

// PlanFree is plan.Lanes.plan_free.
func (x *Lanes) PlanFree() int {
	return max(0, x.PlanLimit-len(x.Planning))
}

// WithRun is these lanes with one more run held (`filling`: in another subscription's lane).
func (x *Lanes) WithRun(number int, providerID string, action string, filling bool /* = False */) *Lanes {
	held := make(map[int]string, len(x.Held)+1)
	for n, p := range x.Held {
		held[n] = p
	}
	held[number] = providerID
	planning := make(map[int]bool, len(x.Planning)+1)
	for n := range x.Planning {
		planning[n] = true
	}
	if action == "plan" {
		planning[number] = true
	}
	fills := make(map[int]bool, len(x.Filling)+1)
	for n := range x.Filling {
		fills[n] = true
	}
	if filling {
		fills[number] = true
	}
	return &Lanes{Limit: x.Limit, Held: held, Planning: planning, PlanLimit: x.PlanLimit,
		MachineLimit: x.MachineLimit, Machine: x.Machine, Shared: x.Shared, Filling: fills}
}

// Describe is plan.Lanes.describe.
func (x *Lanes) Describe() string {
	numbers := make([]int, 0, len(x.Held))
	for n := range x.Held {
		numbers = append(numbers, n)
	}
	slices.Sort(numbers)
	parts := make([]string, 0, len(numbers))
	for _, n := range numbers {
		p := x.Held[n]
		if n == 0 {
			parts = append(parts, "a survey with `"+p+"`")
			continue
		}
		part := fmt.Sprintf("#%d with `%s`", n, p)
		if x.Planning[n] {
			part += " (planning)"
		}
		parts = append(parts, part)
	}
	if len(parts) == 0 {
		return "none"
	}
	return strings.Join(parts, ", ")
}

// SharedLive is SHARED_LIVE: a run of another bot that holds a slot of GitHub's runners: queued
// for one, or on one.
var SharedLive = []string{"queued", "in_progress"}

// OthersOnGithub is how many runs of the other bots (`config.OTHERS`, by their `workflow`) hold
// a slot of the GitHub runners the bots share (#1455). `max_parallel` is one pool, first come
// first served: a slot Squishy holds is one the night bot waits for, and the other way round. A
// night-bot run started with its plan names its runner in its title (bot-night.yml's
// `run-name`), and one on the machine holds no slot; any other live run counts as one. A list
// GitHub cannot read counts as none, so a failed read never stops every lane.
func OthersOnGithub(ctx *hctx.Context) int {
	count := 0
	for _, other := range Others {
		if other.Workflow == "" {
			continue
		}
		for _, status := range SharedLive {
			var runs []*pyjson.Object
			if _, failed := herr.Catch[*herr.GitHubError](func() {
				runs = ctx.GH.ListRuns(other.Workflow, status, 100, false)
			}); failed {
				continue
			}
			for _, run := range runs {
				if eqStr(run.Get("status"), status) &&
					strings.HasSuffix(py.OrStr(run.Get("path"), ""), "/"+other.Workflow) &&
					!strings.Contains(py.OrStr(run.Get("display_title"), ""), " on night-vm-") {
					count++
				}
			}
		}
	}
	return count
}

// WorkingThreads is every thread labelled `bot:working`, closed ones too: a run goes on after
// someone closes its issue, and holds its lane until it ends (deliver then drops its work).
func WorkingThreads(ctx *hctx.Context) []*pyjson.Object {
	return ctx.GH.ListIssues(config.LabelWorking, "all", 300, "")
}

// ReadLanes is the lanes held by runs that are still going. A run that ended without
// delivering holds nothing: housekeeping requeues its item.
func ReadLanes(ctx *hctx.Context, state *pyjson.Object) *Lanes {
	pool := ctx.Cfg.Pool
	machine := map[string]bool{}
	for _, p := range pool.Ordered() {
		if pool.OnMachine(p.ID) {
			machine[p.ID] = true
		}
	}
	lanes := &Lanes{Limit: pool.MaxParallel, Held: map[int]string{}, Planning: map[int]bool{},
		PlanLimit: pool.PlanLanes, MachineLimit: pool.MachineParallel, Machine: machine,
		Filling: map[int]bool{}}
	lanes.Shared = OthersOnGithub(ctx)
	for _, thread := range WorkingThreads(ctx) {
		number := py.MustInt(thread.Get("number"))
		record := recordOf(state, strconv.Itoa(number))
		if RunAlive(ctx, record.Get("run_id")) {
			lanes.Held[number] = py.OrStr(record.Get("provider"), providers.LegacyProvider)
			if eqStr(record.Get("action"), "plan") {
				lanes.Planning[number] = true
			}
			if FillsIn(pool, record) {
				lanes.Filling[number] = true
			}
		}
	}
	survey := state.Obj("suggest")
	if survey.Truthy("provider") && RunAlive(ctx, survey.Get("run_id")) {
		lanes.Held[0] = py.Str(survey.Get("provider"))
	}
	return lanes
}

// FillsIn is: an item's run is in another subscription's lane: it went to its provider's
// fill-in runners (`Provider.fill_in`), which `claim` records as its `runs_on`.
func FillsIn(pool *providers.Pool, record *pyjson.Object) bool {
	provider := pool.Get(py.OrStr(record.Get("provider"), ""))
	return provider != nil && provider.FillIn != nil &&
		eqStr(record.Get("runs_on"), provider.FillIn.RunsOn)
}

// shared is the subscriptions whose lanes `filler` fills, and how many of those lanes runs hold
// now: their own runs, planning ones too, and `filler`'s there.
func shared(pool *providers.Pool, filler *providers.Provider, lanes *Lanes) ([]*providers.Provider, int) {
	var owners []*providers.Provider
	for _, id := range pool.FilledBy(filler) {
		owners = append(owners, pool.Get(id)) // pool.providers[i]: filled_by names only known ones
	}
	held := lanes.FillingBy(filler.ID)
	for _, p := range owners {
		held += lanes.AllBy(p.ID)
	}
	return owners, held
}

// sumLanes is sum(p.lanes for p in owners).
func sumLanes(owners []*providers.Provider) int {
	total := 0
	for _, p := range owners {
		total += p.Lanes
	}
	return total
}

// FillInFree is: `provider` may start a run in the lanes it fills (`Provider.fill_in`): one is
// free, and not kept for a subscription it fills in for that may start work itself now. Muse
// takes gpt's lane on the night box only while gpt is capped, off, outside its hours or backing
// off.
func FillInFree(ctx *hctx.Context, state *pyjson.Object, provider *providers.Provider, lanes *Lanes) bool {
	if provider.FillIn == nil {
		return false
	}
	cfg := ctx.Cfg
	owners, held := shared(cfg.Pool, provider, lanes)
	kept := 0
	for _, p := range owners {
		if providers.Availability(p, state, ctx.Now(), cfg.Timezone, cfg.Secrets, false, true) == "" {
			kept += max(0, p.Lanes-lanes.AllBy(p.ID))
		}
	}
	return sumLanes(owners)-held > kept
}

// FillInFull is: `provider`'s lanes are shared with a subscription that fills them
// (`Provider.fill_in`), and runs hold every one of the shared lanes: it waits for one, even its
// own while a fill-in holds it, so its box never runs more at once than those lanes.
func FillInFull(pool *providers.Pool, provider *providers.Provider, lanes *Lanes) bool {
	filler := pool.FillerOf(provider.ID)
	if filler == nil {
		return false
	}
	owners, held := shared(pool, filler, lanes)
	return held >= sumLanes(owners)
}

// RunnerFor is the runner label `provider`'s next run takes: its own (`runs_on`) while one of
// its lanes is free, else its fill-in runners' (`fill_in`) while it may fill a lane
// (`fill_in_free`; a suggestion survey never does), else "".
func RunnerFor(ctx *hctx.Context, state *pyjson.Object, provider *providers.Provider, lanes *Lanes, role string) string {
	if !lanes.Full(provider) {
		return provider.RunsOn
	}
	if role != "suggest" && FillInFree(ctx, state, provider, lanes) {
		return provider.FillIn.RunsOn
	}
	return ""
}

// quietHolds is the quiet check holding `provider` back: `provider.quiet_check and
// cfg.quiet.enabled and not forced and quiet_ok not in (ANY_QUIET, provider.id)`.
func quietHolds(cfg *config.Config, provider *providers.Provider, forced bool, quietOK string) bool {
	return provider.QuietCheck && cfg.Quiet.Enabled && !forced &&
		quietOK != AnyQuiet && quietOK != provider.ID
}

// Usable is plan._usable.
func Usable(ctx *hctx.Context, state *pyjson.Object, provider *providers.Provider, lanes *Lanes, role string, forced bool, quietOK string) bool {
	cfg := ctx.Cfg
	if !slices.Contains(provider.Roles, role) || !lanes.RoomFor(provider.ID) {
		return false
	}
	if MachineFull(cfg.Pool, provider, lanes) || FillInFull(cfg.Pool, provider, lanes) {
		return false
	}
	if RunnerFor(ctx, state, provider, lanes, role) == "" {
		return false
	}
	if quietHolds(cfg, provider, forced, quietOK) {
		return false
	}
	// A build or a revision runs long: it starts only with `start_headroom` under each cap.
	return providers.Availability(provider, state, ctx.Now(), cfg.Timezone, cfg.Secrets,
		forced, role == "build" || role == "revise") == ""
}
