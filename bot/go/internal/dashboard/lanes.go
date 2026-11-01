package dashboard

// ------------------------------------------------------------------ the lanes

import (
	"fmt"
	"slices"
	"sort"
	"strconv"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/waiting"
)

// LaneOrder is LANE_ORDER: the lanes table's order after the Claude accounts, which come first by
// number: the AWS boxes' subscriptions, as the owner reads them. Any other follows by name, and the
// other bots' last.
var LaneOrder = []string{"agy", "gpt", "muse"}

// LanesMark is LANES_MARK: how a bot's section carries its lanes to the night bot's table
// (`lanes_mark`): JSON in an HTML comment, which GitHub draws for nobody.
var LanesMark = "<!-- jackioh-bot:lanes "

// ClaudeSecret is CLAUDE_SECRET: what the Claude accounts' secrets are named
// (`providers.SECRETS`): one account each.
var ClaudeSecret = "CLAUDE_CODE_OAUTH_TOKEN"

// lanesWhere is _lanes_where.
func lanesWhere(pool *providers.Pool, provider *providers.Provider) string {
	if !pool.OnMachine(provider.ID) {
		return "GitHub's runners"
	}
	var places []string
	for _, slot := range BoxSlots {
		if slices.Contains(slot.Owners, provider.ID) || slices.Contains(slot.Extras, provider.ID) {
			places = append(places, "the "+slot.Place)
		}
	}
	if len(places) == 0 {
		return "`" + provider.RunsOn + "`"
	}
	return strings.Join(places, " and ")
}

// LaneRows is lane_rows: each subscription's lanes, in `LANE_ORDER`: how many it has, how many of
// them its runs hold now (`plan.Lanes.used`, what the planner holds to them), whether it is switched
// off or may start a build or a plan now (`waiting.held_by`: its usage and hours; nothing while the
// bot is `halted`), its secret and where it runs. A fill-in's runs (`FillIn`) count in the lanes
// they fill, the first of its `for` first; a planning run the planning lane holds counts beside its
// lanes (`planning`).
//
// Each row is an object with the Python's keys in the Python's order, since `lanes_mark` writes
// them as JSON.
func LaneRows(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object,
	halted string /* = "" */) []*pyjson.Object {
	pool, cfg := ctx.Cfg.Pool, ctx.Cfg
	lanes := waiting.LanesOf(ctx, state, live)
	rows := pyjson.New() // provider id -> row
	for _, p := range pool.Ordered() {
		off := !p.Enabled || providers.SwitchedOffByDate(p, ctx.Now(), cfg.Timezone) ||
			(p.Login == "secret" && isFalse(cfg.Secrets.Has(p.Secret)))
		counted := p.QuietCheck || p.Limits.Stops().Len() > 0
		secret := ""
		if p.Login == "secret" {
			secret = p.Secret
		}
		where, github := lanesWhere(pool, p), !pool.OnMachine(p.ID)
		startable := !(off || halted != "") && waiting.HeldBy(ctx, state, p, false, true) == nil
		used := lanes.Used(p)
		planning := 0
		if !counted {
			planning = lanes.PlanningBy(p.ID)
		}
		rows.Set(p.ID, pyjson.New("id", p.ID, "bot", "", "claude", p.Family == "claude",
			"where", where, "github", github, "secret", secret, "startable", startable,
			"lanes", p.Lanes, "used", used, "off", off, "planning", planning,
			"runs", lanes.AllBy(p.ID), "filling", lanes.FillingBy(p.ID),
			"fills", pyjson.ListOf(pool.FilledBy(p)), "filled", 0, "filler", ""))
	}
	for _, filler := range pool.Ordered() {
		left := rows.Obj(filler.ID).Int("filling")
		for _, owner := range pool.FilledBy(filler) {
			row := rows.Obj(owner)
			if row == nil {
				panic(fmt.Errorf("KeyError: %s", py.Repr(owner)))
			}
			took := min(left, max(0, row.Int("lanes")-row.Int("used")))
			fillerID := ""
			if took != 0 {
				fillerID = filler.ID
			}
			row.Set("filled", took)
			row.Set("filler", fillerID)
			left -= took
		}
	}

	type key struct {
		group, rank int
		id          string
	}
	order := func(row *pyjson.Object) key {
		id := row.Str("id")
		number := strings.TrimPrefix(id, "claude-")
		if row.Truthy("claude") && py.IsDecimal(number) {
			n, _ := strconv.Atoi(number)
			return key{0, n, ""}
		}
		if i := slices.Index(LaneOrder, id); i >= 0 {
			return key{1, i, ""}
		}
		return key{2, 0, id}
	}
	var out []*pyjson.Object
	rows.Items(func(_ string, v any) bool {
		out = append(out, v.(*pyjson.Object))
		return true
	})
	keys := make(map[*pyjson.Object]key, len(out))
	for _, row := range out {
		keys[row] = order(row)
	}
	sort.SliceStable(out, func(i, j int) bool {
		a, b := keys[out[i]], keys[out[j]]
		if a.group != b.group {
			return a.group < b.group
		}
		if a.rank != b.rank {
			return a.rank < b.rank
		}
		return a.id < b.id
	})
	return out
}

// isFalse is `x is False` for the `bool | None` of `Secrets.has`.
func isFalse(b *bool) bool { return b != nil && !*b }

// LanesMark_ is lanes_mark: this bot's lanes for the night bot's table (`marked_lanes`), named for
// this bot.
func LanesMark_(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object,
	halted string /* = "" */) string {
	var rows []any
	for _, row := range LaneRows(ctx, state, live, halted) {
		rows = append(rows, row.Copy().Set("bot", config.Title))
	}
	if rows == nil {
		rows = []any{}
	}
	// No `>` inside, so nothing in it can close the comment.
	return LanesMark + strings.ReplaceAll(pyjson.Dumps(rows), ">", `>`) + " -->"
}

// MarkedLanes is marked_lanes: the other bots' lanes their sections carry (`lanes_mark`), in their
// order. A section with none, or with one that cannot be read, adds no row.
func MarkedLanes(parts []string) []*pyjson.Object {
	var found []*pyjson.Object
	for _, part := range parts {
		start := strings.Index(part, LanesMark)
		if start < 0 {
			continue
		}
		end := strings.Index(part[start:], " -->")
		if end < 0 {
			continue // part.index's ValueError
		}
		end += start
		text := ""
		if end > start+len(LanesMark) {
			text = part[start+len(LanesMark) : end]
		}
		data, err := pyjson.LoadsString(text)
		if err != nil {
			continue
		}
		if rows, ok := markedRows(data); ok {
			found = append(found, rows...)
		}
	}
	return found
}

// markedRows is `marked_lanes`'s comprehension over one section's JSON: every row, or false
// where the Python's raises (ValueError, TypeError, KeyError, AttributeError) and the whole
// section adds none.
func markedRows(data any) ([]*pyjson.Object, bool) {
	var items []any
	switch x := data.(type) {
	case []any:
		items = x
	case *pyjson.Object: // iterating a dict yields its keys, and `"key"["id"]` raises
		return nil, x.Len() == 0
	case string:
		return nil, x == ""
	default:
		return nil, false
	}
	var rows []*pyjson.Object
	for _, item := range items {
		r, ok := item.(*pyjson.Object)
		if !ok || r == nil {
			return nil, false
		}
		id, has := r.Lookup("id")
		if !has {
			return nil, false
		}
		row := pyjson.New("id", py.Str(id), "bot", py.OrStr(r.Get("bot"), ""),
			"claude", r.Truthy("claude"), "where", py.OrStr(r.Get("where"), ""),
			"github", r.Truthy("github"), "off", r.Truthy("off"),
			"secret", py.OrStr(r.Get("secret"), ""), "startable", r.Truthy("startable"))
		for _, k := range []string{"lanes", "used", "planning", "runs", "filling", "filled"} {
			n := 0
			if v := r.Get(k); pyjson.Truthy(v) {
				var err error
				if n, err = py.Int(v); err != nil {
					return nil, false
				}
			}
			row.Set(k, n)
		}
		fills, ok := iterStrs(r.Get("fills"))
		if !ok {
			return nil, false
		}
		row.Set("fills", fills)
		row.Set("filler", py.OrStr(r.Get("filler"), ""))
		rows = append(rows, row)
	}
	return rows, true
}

// iterStrs is `[str(o) for o in v or []]`: a list's items, a string's characters or a dict's keys;
// false where iterating raises TypeError.
func iterStrs(v any) ([]any, bool) {
	out := []any{}
	if !pyjson.Truthy(v) {
		return out, true
	}
	switch x := v.(type) {
	case []any:
		for _, o := range x {
			out = append(out, py.Str(o))
		}
	case string:
		for _, r := range x {
			out = append(out, string(r))
		}
	case *pyjson.Object:
		for _, k := range x.Keys() {
			out = append(out, k)
		}
	default:
		return nil, false
	}
	return out, true
}

// Open is _open.
func Open(row *pyjson.Object) int {
	if row.Truthy("off") {
		return 0
	}
	return max(0, row.Int("lanes")-row.Int("used")-row.Int("filled"))
}

// inUse is _in_use.
func inUse(row *pyjson.Object) string {
	var fills []string
	for _, owner := range pyjson.Strings(row.Get("fills")) {
		fills = append(fills, "`"+owner+"`'s")
	}
	var notes []string
	if filled := row.Int("filled"); filled != 0 {
		notes = append(notes, fmt.Sprintf("%d of them `%s`'s", filled, row.Str("filler")))
	}
	if filling := row.Int("filling"); filling != 0 {
		notes = append(notes, fmt.Sprintf("+%d filling in %s", filling, strings.Join(fills, " or ")))
	}
	if planning := row.Int("planning"); planning != 0 {
		notes = append(notes, fmt.Sprintf("+%d planning, on the planning lane", planning))
	}
	text := strconv.Itoa(row.Int("used") + row.Int("filled"))
	if len(notes) > 0 {
		text += " <sub>(" + strings.Join(notes, ", ") + ")</sub>"
	}
	return text
}

// LanesTable is lanes_table: how many lanes each subscription has, how many its runs hold and how
// many are open: the night bot's, the Claude accounts first by number (`lane_rows`), then each other
// bot's from its section (`marked_lanes`), then the totals, the Claude accounts' and everyone's.
// Under them, the limits on the lanes: an account's own, the Claude secrets the workflows hand over,
// the runs GitHub's runners, the planning lane and the AWS boxes take at once, usage and hours, and
// how many more Claude runs there is room for now.
func LanesTable(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object,
	others []string /* = () */, halted string /* = "" */) []string {
	rows := slices.Concat(LaneRows(ctx, state, live, halted), MarkedLanes(others))
	lines := []string{"| Subscription | Runs on | Lanes | In use | Open |", "|---|---|---|---|---|"}
	for _, row := range rows {
		name := "`" + row.Str("id") + "`"
		if row.Str("bot") != "" {
			name += " (" + row.Str("bot") + ")"
		}
		open := "⚫ off"
		if !row.Truthy("off") {
			open = strconv.Itoa(Open(row))
		}
		lines = append(lines, fmt.Sprintf("| %s | %s | %d | %s | %s |", name, row.Str("where"),
			row.Int("lanes"), inUse(row), open))
	}
	var claude []*pyjson.Object
	for _, r := range rows {
		if r.Truthy("claude") {
			claude = append(claude, r)
		}
	}
	for i, total := range []struct {
		label  string
		picked []*pyjson.Object
	}{{fmt.Sprintf("All %d Claude accounts", len(claude)), claude},
		{fmt.Sprintf("All %d subscriptions", len(rows)), rows}} {
		if len(total.picked) == 0 || (i == 1 && len(claude) == len(rows)) {
			continue
		}
		off, lanesSum, usedSum, openSum := 0, 0, 0, 0
		for _, r := range total.picked {
			if r.Truthy("off") {
				off++
			}
			lanesSum += r.Int("lanes")
			usedSum += r.Int("used") + r.Int("filled")
			openSum += Open(r)
		}
		offText := ""
		if off != 0 {
			offText = fmt.Sprintf(" (%d off)", off)
		}
		lines = append(lines, fmt.Sprintf("| **%s**%s | | **%d** | **%d** | **%d** |", total.label,
			offText, lanesSum, usedSum, openSum))
	}
	lines = append(lines, "", "**The limits on the lanes**", "")
	return append(lines, LaneLimits(ctx, state, live, rows)...)
}

// LaneLimits is lane_limits: what holds the lanes, as a list (`lanes_table`), each limit with how
// much of it is held.
func LaneLimits(ctx *hctx.Context, state *pyjson.Object, live *pyjson.Object,
	rows []*pyjson.Object) []string {
	pool := ctx.Cfg.Pool
	botName := config.Name
	var claude, github []*pyjson.Object
	for _, r := range rows {
		if r.Truthy("claude") {
			claude = append(claude, r)
		}
	}
	for _, r := range rows {
		if r.Truthy("github") {
			github = append(github, r)
		}
	}
	hostedRuns := 0
	for _, l := range lanesIn(live) {
		if !pool.OnMachine(l.provider) {
			hostedRuns++
		}
	}
	runs := pyjson.New(botName, hostedRuns) // bot -> its runs on GitHub's runners, in order
	for _, row := range github {
		if bot := row.Str("bot"); bot != "" {
			runs.Set(bot, runs.Int(bot)+row.Int("runs"))
		}
	}
	held, onGitHub := 0, 0
	runs.Items(func(_ string, n any) bool {
		held += n.(int)
		return true
	})
	for _, r := range github {
		onGitHub += r.Int("lanes")
	}
	var secrets []string
	for _, name := range providers.SecretNames {
		if strings.HasPrefix(name, ClaudeSecret) {
			secrets = append(secrets, name)
		}
	}
	var namedBots []string // bot -> the Claude secrets its accounts name, in order
	named := map[string]map[string]bool{}
	for _, row := range claude {
		if secret := row.Str("secret"); slices.Contains(secrets, secret) {
			bot := row.Str("bot")
			if bot == "" {
				bot = botName
			}
			if named[bot] == nil {
				named[bot] = map[string]bool{}
				namedBots = append(namedBots, bot)
			}
			named[bot][secret] = true
		}
	}
	var free []string
	for _, name := range secrets {
		taken := false
		for _, bot := range namedBots {
			taken = taken || named[bot][name]
		}
		if !taken {
			free = append(free, "`"+name+"`")
		}
	}
	there := "Claude lanes"
	for _, r := range github {
		if !r.Truthy("claude") {
			there = "lanes there"
			break
		}
	}
	var perBot []string
	for _, bot := range namedBots {
		perBot = append(perBot, fmt.Sprintf("%d %s's", len(named[bot]), bot))
	}
	accounts := fmt.Sprintf("- **Accounts:** the workflows hand over %d Claude secrets "+
		"(`providers.SECRETS`), one an account: ", len(secrets)) + strings.Join(perBot, ", ")
	if len(free) > 0 {
		accounts += "; free: " + strings.Join(free, ", ") + "."
	} else {
		accounts += ", so none is free: another account needs a new secret there and in the workflows."
	}
	var runsText []string
	runs.Items(func(bot string, n any) bool {
		runsText = append(runsText, fmt.Sprintf("%d %s's", n.(int), bot))
		return true
	})
	atOnce := fmt.Sprintf("- **At once:** GitHub's runners take %d of the bots' runs, planning "+
		"runs too (%d now: ", pool.MaxParallel, held) + strings.Join(runsText, ", ") +
		fmt.Sprintf("), the organization's %d hosted jobs less the %d kept for CI (#1455).",
			HostedRunners, HostedRunners-pool.MaxParallel)
	if onGitHub > pool.MaxParallel {
		atOnce += fmt.Sprintf(" So the %d %s never all run at once: past %d, a lane lets work go "+
			"to an account with room while others are capped or outside their hours.", onGitHub,
			there, pool.MaxParallel)
	}
	limits := []string{
		"- **Each account:** its `lanes` in its bot's `providers.json`, 1 or more; the code sets " +
			"no top. Each lane is a run on a GitHub runner of its own, and its runs share the " +
			"account's usage: each reads it before it starts and watches it while it works, so " +
			"together they pass its cap by at most one reading each.",
		accounts, atOnce}
	if pool.PlanLanes != 0 {
		planning := 0
		for _, l := range lanesIn(live) {
			if l.number != 0 && pyjson.Equal(status.Record(state, l.number).Get("action"), "plan") {
				planning++
			}
		}
		limits = append(limits, fmt.Sprintf("- **Planning:** the planning lane runs %d plans at once "+
			"beside the accounts' own lanes (%d now), on those same runners.", pool.PlanLanes, planning))
	}
	if pool.MachineParallel != 0 {
		onMachine := 0
		for _, l := range lanesIn(live) {
			if pool.OnMachine(l.provider) {
				onMachine++
			}
		}
		limits = append(limits, fmt.Sprintf("- **The AWS boxes:** %d runs at once (%d now), apart "+
			"from GitHub's runners.", pool.MachineParallel, onMachine))
	}
	startable := 0
	for _, r := range claude {
		if r.Truthy("startable") {
			startable += Open(r)
		}
	}
	room := max(0, pool.MaxParallel-held)
	lanesAre, runsWord := "lanes are", "runs"
	if startable == 1 {
		lanesAre = "lane is"
	}
	if min(startable, room) == 1 {
		runsWord = "run"
	}
	return append(limits,
		"- **Usage and hours:** an open lane takes work only while its subscription is under "+
			"its caps and in its hours (🔋 Subscriptions); a capped account counts its planning run "+
			"in its own lanes.",
		fmt.Sprintf("- **Now:** %d Claude %s open on accounts that may start work, and GitHub's "+
			"runners have %d free, so there is room for %d more Claude %s.", startable, lanesAre, room,
			min(startable, room), runsWord))
}
