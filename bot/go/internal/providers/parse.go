package providers

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"regexp"
	"slices"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// configError raises ConfigError.
func configError(format string, args ...any) {
	herr.Raise(&herr.ConfigError{Msg: fmt.Sprintf(format, args...)})
}

// fraction is _fraction: nil for None.
func fraction(value any, where string) *float64 {
	if value == nil {
		return nil
	}
	number := py.MustFloat(value)
	if !(0.0 < number && number <= 1.0) {
		configError("%s: %s is outside (0, 1]", where, py.Str(value))
	}
	return &number
}

// schedule is _schedule.
func schedule(raw any, where string) *Schedule {
	obj, ok := mapping(raw)
	if !ok {
		configError("%s: expected an object", where)
	}
	mode := py.Str(obj.GetOr("mode", ""))
	if mode == "always" {
		return &Schedule{Mode: "always"}
	}
	if mode == "window" {
		var start, end string
		var exc string
		for _, key := range []string{"start", "end"} {
			if !obj.Has(key) {
				exc = py.Repr(key) // str(KeyError('start')) is the key's repr
				break
			}
		}
		if exc == "" {
			start, end = py.Str(obj.Get("start")), py.Str(obj.Get("end"))
			// clock.parse_hhmm raises ValueError, whose text goes into the message.
			if err := herr.Try(func() {
				for _, text := range []string{start, end} {
					_, perr := clock.ParseHhmm(text)
					herr.Must(perr)
				}
			}); err != nil {
				exc = err.Error()
			}
		}
		if exc != "" {
			configError("%s: a window needs start and end as HH:MM (%s)", where, exc)
		}
		return &Schedule{Mode: "window", Start: start, End: end}
	}
	configError("%s.mode: %s is not always or window", where, py.Repr(mode))
	return nil
}

// headroom is _headroom: `start_headroom`, the pool's or one subscription's: a share of each
// window, 0 included (a build, a revision or a plan may start right up to that cap).
func headroom(raw any, where string) *pyjson.Object {
	if noneOrEmpty(raw) {
		return pyjson.New()
	}
	obj, ok := mapping(raw)
	if !ok {
		configError(`%s: expected an object such as {"five_hour": 0.15}`, where)
	}
	if unknown := unknownKeys(obj, windowOrder); len(unknown) > 0 {
		configError("%s: unknown windows %s", where, strings.Join(unknown, ", "))
	}
	shares := pyjson.New()
	obj.Items(func(window string, share any) bool {
		number := py.MustFloat(share)
		if !(0.0 <= number && number < 1.0) {
			configError("%s.%s: %s is outside [0, 1)", where, window, py.Str(share))
		}
		shares.Set(window, number)
		return true
	})
	return shares
}

// limits is _limits.
func limits(raw any, where string) *Limits {
	obj, ok := mapping(raw)
	if !ok {
		configError("%s: expected an object", where)
	}
	mode := py.Str(obj.GetOr("mode", ""))
	if mode == "none" {
		return &Limits{Mode: "none", Headroom: pyjson.New()}
	}
	if mode != "caps" {
		configError("%s.mode: %s is not caps or none", where, py.Repr(mode))
	}
	minutes := func(key string) *int {
		value := obj.Get(key)
		if value == nil {
			return nil
		}
		if py.MustInt(value) < 1 {
			configError("%s.%s: must be at least 1", where, key)
		}
		n := py.MustInt(value)
		return &n
	}
	fiveHour := fraction(obj.Get("five_hour"), where+".five_hour")
	sevenDay := fraction(obj.Get("seven_day"), where+".seven_day")
	fiveHourMinutes := minutes("five_hour_minutes")
	sevenDayMinutes := minutes("seven_day_minutes")
	found := &Limits{Mode: "caps", FiveHour: fiveHour, SevenDay: sevenDay,
		FiveHourMinutes: fiveHourMinutes, SevenDayMinutes: sevenDayMinutes, Headroom: pyjson.New()}
	if found.Stops().Len() == 0 && found.Budgets().Len() == 0 {
		configError("%s: caps needs at least one of five_hour, seven_day, "+
			"five_hour_minutes, seven_day_minutes", where)
	}
	return found
}

// providerKeys is _PROVIDER_KEYS.
var providerKeys = []string{"enabled", "cli", "family", "model", "effort", "tier", "secret", "schedule",
	"limits", "quiet_check", "roles", "env", "note", "login", "runs_on",
	"self_check", "extra_models", "off_from", "off_reason", "usage_since",
	"lanes", "build_last", "easy_first", "off_hours", "fix_effort",
	"review_effort", "start_headroom", "fill_in", "last_hour", "late_week"}

// tier is _tier.
func tier(value any, where string) string {
	found := ""
	if pyjson.Truthy(value) {
		found = py.Str(value)
	}
	if !slices.Contains(Tiers, found) {
		configError("%s: %s is not one of %s", where, py.Repr(found), strings.Join(Tiers, ", "))
	}
	return found
}

// extraModels is _extra_models.
func extraModels(raw any, effort string, where string) []*ExtraModel {
	if raw == nil {
		return nil
	}
	list, ok := raw.([]any)
	if !ok {
		configError("%s: expected a list", where)
	}
	var found []*ExtraModel
	for i, item := range list {
		entry, ok := mapping(item)
		if !ok || !entry.Truthy("model") {
			configError("%s[%d]: needs a model and a tier", where, i)
		}
		if unknown := unknownKeys(entry, []string{"model", "effort", "tier", "note", "takes_over"}); len(unknown) > 0 {
			configError("%s[%d]: unknown keys %s", where, i, strings.Join(unknown, ", "))
		}
		model := py.Str(entry.Get("model"))
		modelEffort := py.Str(entry.GetOr("effort", effort))
		modelTier := tier(entry.Get("tier"), fmt.Sprintf("%s[%d].tier", where, i))
		found = append(found, &ExtraModel{model, modelEffort, modelTier, py.OrStr(entry.Get("takes_over"), "")})
	}
	return found
}

// fillIn is _fill_in.
func fillIn(raw any, where string) *FillIn {
	if raw == nil {
		return nil
	}
	obj, ok := mapping(raw)
	if !ok || !obj.Truthy("runs_on") || !obj.Truthy("for") {
		configError("%s: needs runs_on and for (the subscriptions whose lanes it fills)", where)
	}
	if unknown := unknownKeys(obj, []string{"runs_on", "for", "note"}); len(unknown) > 0 {
		configError("%s: unknown keys %s", where, strings.Join(unknown, ", "))
	}
	runsOn := py.Str(obj.Get("runs_on"))
	if !label.MatchString(runsOn) || Hosted(runsOn) {
		configError("%s.runs_on: %s is not a runner label on the machine", where, py.Repr(runsOn))
	}
	list, isList := obj.Get("for").([]any)
	var of []string
	for _, i := range list {
		if s, ok := i.(string); ok && s != "" {
			of = append(of, s)
		} else {
			isList = false
			break
		}
	}
	if !isList {
		configError("%s.for: expected a list of subscriptions", where)
	}
	return &FillIn{RunsOn: runsOn, Of: of}
}

// lateWeekFrom is the pattern `late_week`'s `from` must match:
// `\s*([A-Za-z]{3})[A-Za-z]*\s+(\d{1,2}:\d{2})\s*`, with Python's Unicode `\s` and `\d`.
var lateWeekFrom = regexp.MustCompile(`\A` + py.ReS + `*([A-Za-z]{3})[A-Za-z]*` + py.ReS +
	`+(\p{Nd}{1,2}:\p{Nd}{2})` + py.ReS + `*\z`)

// lateWeek is _late_week.
func lateWeek(raw any, where string) *LateWeek {
	if raw == nil {
		return nil
	}
	obj, ok := mapping(raw)
	if !ok || !obj.Truthy("from") || obj.Get("seven_day") == nil {
		configError(`%s: needs from ("Sun 17:00") and seven_day`, where)
	}
	if unknown := unknownKeys(obj, []string{"from", "seven_day", "five_hour", "last_hour", "note"}); len(unknown) > 0 {
		configError("%s: unknown keys %s", where, strings.Join(unknown, ", "))
	}
	found := lateWeekFrom.FindStringSubmatch(py.Str(obj.Get("from")))
	if found == nil || !slices.Contains(Weekdays, strings.ToLower(found[1])) {
		configError(`%s.from: %s is not a weekday and a time ("Sun 17:00")`, where, py.Repr(obj.Get("from")))
	}
	var start clock.TimeOfDay
	if err := herr.Try(func() {
		var perr error
		start, perr = clock.ParseHhmm(found[2])
		herr.Must(perr)
	}); err != nil {
		configError("%s.from: %s: %s", where, py.Repr(obj.Get("from")), err.Error())
	}
	return &LateWeek{
		Weekday:  slices.Index(Weekdays, strings.ToLower(found[1])),
		Start:    start,
		SevenDay: *fraction(obj.Get("seven_day"), where+".seven_day"),
		FiveHour: fraction(obj.Get("five_hour"), where+".five_hour"),
		LastHour: pyjson.Truthy(obj.GetOr("last_hour", false)),
	}
}

// tiersOf is _tiers.
func tiersOf(raw any) *pyjson.Object {
	where := ProvidersPath + ": tiers"
	obj, ok := mapping(raw)
	if !ok {
		configError("%s: expected an object with %s", where, strings.Join(Tiers, ", "))
	}
	if unknown := unknownKeys(obj, Tiers); len(unknown) > 0 {
		configError("%s: unknown tiers %s", where, strings.Join(unknown, ", "))
	}
	found := pyjson.New()
	var models []string
	for _, name := range Tiers {
		entries := []*TierEntry{}
		var listed any = []any{}
		if obj.Truthy(name) {
			listed = obj.Get(name)
		}
		for i, item := range py.MustIter(listed) {
			if s, ok := item.(string); ok {
				item = pyjson.New("model", s)
			}
			entry, ok := mapping(item)
			if !ok || !entry.Truthy("model") {
				configError("%s.%s[%d]: needs a model", where, name, i)
			}
			if extra := unknownKeys(entry, []string{"model", "note"}); len(extra) > 0 {
				configError("%s.%s[%d]: unknown keys %s", where, name, i, strings.Join(extra, ", "))
			}
			entries = append(entries, &TierEntry{py.Str(entry.Get("model"))})
			models = append(models, py.Str(entry.Get("model")))
		}
		found.Set(name, entries)
	}
	if twice := repeated(models); len(twice) > 0 {
		configError("%s: %s is in more than one place; a model has one tier", where, strings.Join(twice, ", "))
	}
	return found
}

// provider is _provider.
func provider(name string, raw any) *Provider {
	where := "providers." + name
	obj, ok := mapping(raw)
	if !ok {
		configError("%s: expected an object", where)
	}
	if unknown := unknownKeys(obj, providerKeys); len(unknown) > 0 {
		configError("%s: unknown keys %s", where, strings.Join(unknown, ", "))
	}
	for _, key := range []string{"cli", "family", "model", "tier", "schedule", "limits"} {
		if !obj.Has(key) {
			configError("%s: missing %s", where, key)
		}
	}
	cli := py.Str(obj.Get("cli"))
	if !slices.Contains(Clis, cli) {
		configError("%s.cli: %s is not one of %s", where, py.Repr(cli), strings.Join(Clis, ", "))
	}
	login := py.Str(obj.GetOr("login", "secret"))
	if !slices.Contains(Logins, login) {
		configError("%s.login: %s is not one of %s", where, py.Repr(login), strings.Join(Logins, ", "))
	}
	secret := py.OrStr(obj.Get("secret"), "")
	if login == "secret" {
		if !slices.Contains(SecretClis, cli) {
			configError(`%s: the %s CLI logs in on the machine only ("login": "machine")`, where, cli)
		}
		if !slices.Contains(SecretNames, secret) {
			configError("%s.secret: %s is not one the workflows hand over (%s)", where,
				py.Repr(secret), strings.Join(SecretNames, ", "))
		}
	} else if secret != "" {
		configError("%s.secret: a machine login takes no secret", where)
	}
	runsOn := py.Str(obj.GetOr("runs_on", "ubuntu-latest"))
	if !label.MatchString(runsOn) {
		configError("%s.runs_on: %s is not a runner label", where, py.Repr(runsOn))
	}
	if login == "machine" && Hosted(runsOn) {
		configError("%s: a machine login needs runs_on to be its runner on the machine "+
			"(night-vm-%s), not GitHub's %s", where, name, runsOn)
	}
	var roles []string
	if rawRoles, ok := obj.Lookup("roles"); ok {
		for _, r := range py.MustIter(rawRoles) {
			roles = append(roles, py.Str(r))
		}
	} else {
		roles = append(roles, Roles...)
	}
	var bad []string
	for _, r := range roles {
		if !slices.Contains(Roles, r) {
			bad = append(bad, r)
		}
	}
	if len(bad) > 0 {
		configError("%s.roles: unknown %s", where, strings.Join(bad, ", "))
	}
	var env any = pyjson.New()
	if obj.Truthy("env") {
		env = obj.Get("env")
	}
	envObj, ok := mapping(env)
	if !ok {
		configError("%s.env: expected an object", where)
	}
	if obj.Truthy("quiet_check") && cli != "claude" {
		configError("%s.quiet_check: only a Claude account can be checked for quiet "+
			"(the check reads Claude's usage)", where)
	}
	found := limits(obj.Get("limits"), where+".limits")
	if obj.Truthy("last_hour") && !found.Stops().Has("five_hour") {
		configError("%s.last_hour: it lifts a 5-hour cap, and %s has none", where, name)
	}
	late := lateWeek(obj.Get("late_week"), where+".late_week")
	if late != nil {
		late.Caps().Items(func(window string, limit any) bool {
			own := 1.0
			if stop := found.Stops().Get(window); pyjson.Truthy(stop) {
				own = stop.(float64)
			}
			if limit.(float64) <= own {
				configError("%s.late_week: it raises a %s cap, and %s's is not under %s", where,
					WindowNames[window], name, py.Percent(limit.(float64), 0))
			}
			return true
		})
		if late.LastHour && !found.Stops().Has("five_hour") {
			configError("%s.late_week.last_hour: it lifts a 5-hour cap, and %s has none", where, name)
		}
	}
	// The keyword arguments in the Python's order, since each may raise.
	p := &Provider{ID: name}
	p.Enabled = pyjson.Truthy(obj.GetOr("enabled", true))
	p.CLI = cli
	p.Family = py.Str(obj.Get("family"))
	p.Model = py.Str(obj.Get("model"))
	p.Effort = py.Str(obj.GetOr("effort", ""))
	p.Tier = tier(obj.Get("tier"), where+".tier")
	p.Secret = secret
	p.Schedule = schedule(obj.Get("schedule"), where+".schedule")
	withHeadroom := *found
	withHeadroom.Headroom = headroom(obj.Get("start_headroom"), where+".start_headroom")
	p.Limits = &withHeadroom
	p.QuietCheck = pyjson.Truthy(obj.GetOr("quiet_check", false))
	p.Roles = roles
	p.Env = map[string]string{}
	envObj.Items(func(k string, v any) bool {
		p.Env[k] = py.Str(v)
		return true
	})
	p.Login = login
	p.RunsOn = runsOn
	p.OffFrom = offFrom(obj.Get("off_from"), where+".off_from")
	p.OffReason = py.OrStr(obj.Get("off_reason"), "")
	p.UsageSince = usageSince(obj.Get("usage_since"), where+".usage_since")
	p.Lanes = lanes(obj.GetOr("lanes", 1), where+".lanes")
	p.FillIn = fillIn(obj.Get("fill_in"), where+".fill_in")
	p.SelfCheck = pyjson.Truthy(obj.GetOr("self_check", false))
	p.BuildLast = pyjson.Truthy(obj.GetOr("build_last", false))
	p.EasyFirst = pyjson.Truthy(obj.GetOr("easy_first", false))
	p.OffHours = offHours(obj.Get("off_hours"), where+".off_hours")
	p.FixEffort = py.OrStr(obj.Get("fix_effort"), "")
	p.ReviewEffort = py.OrStr(obj.Get("review_effort"), "")
	p.ExtraModels = extraModels(obj.Get("extra_models"), py.Str(obj.GetOr("effort", "")),
		where+".extra_models")
	p.LastHour = pyjson.Truthy(obj.GetOr("last_hour", false))
	p.LateWeek = late
	return p
}

// Parse is parse: a `Pool` from the JSON of `.harness/providers.json`. Raises `ConfigError`.
func Parse(raw any) *Pool {
	obj, ok := mapping(raw)
	if !ok {
		configError("%s must hold a JSON object", ProvidersPath)
	}
	providers := pyjson.New()
	if obj.Truthy("providers") {
		listed, ok := mapping(obj.Get("providers"))
		if !ok {
			panic(fmt.Errorf("TypeError: dict() of %s", py.Repr(obj.Get("providers"))))
		}
		listed.Items(func(k string, v any) bool {
			providers.Set(k, provider(k, v))
			return true
		})
	}
	poolHeadroom := headroom(obj.Get("start_headroom"), ProvidersPath+".start_headroom")
	// A subscription's own `start_headroom` wins, window by window, over the pool's.
	merged := pyjson.New()
	providers.Items(func(k string, v any) bool {
		p := v.(*Provider)
		if p.Limits.Stops().Len() > 0 {
			l := *p.Limits
			l.Headroom = poolHeadroom.Copy().Update(p.Limits.Headroom)
			c := *p
			c.Limits = &l
			p = &c
		}
		merged.Set(k, p)
		return true
	})
	providers = merged
	if providers.Len() == 0 {
		configError("%s: at least one provider is required", ProvidersPath)
	}
	pool := &Pool{Providers: providers}
	all := pool.providerList()
	var secrets []string
	for _, p := range all {
		if p.Login == "secret" {
			secrets = append(secrets, p.Secret)
		}
	}
	if clash := repeated(secrets); len(clash) > 0 {
		configError("%s: two providers share the secret %s", ProvidersPath, strings.Join(clash, ", "))
	}
	// A runner on the machine is one Linux user with one home: a machine login lives there, and a
	// second provider on the same runner would run in the first one's home, with its login.
	for _, p := range all {
		var shared []string
		for _, q := range all {
			if q.ID != p.ID && q.RunsOn == p.RunsOn {
				shared = append(shared, q.ID)
			}
		}
		sort.Strings(shared)
		if len(shared) > 0 && !Hosted(p.RunsOn) {
			configError("%s: %s and %s share the runner %s; each provider on the machine has its own",
				ProvidersPath, p.ID, strings.Join(shared, ", "), p.RunsOn)
		}
	}
	// A fill-in (`FillIn`) runs on runners of its own, in the lanes of other machine
	// subscriptions, each filled by one subscription at most.
	var labels []string
	for _, q := range all {
		labels = append(labels, q.RunsOn)
	}
	var filled []string
	for _, p := range all {
		if p.FillIn == nil {
			continue
		}
		where := fmt.Sprintf("%s: %s's fill_in", ProvidersPath, p.ID)
		if Hosted(p.RunsOn) {
			configError("%s: only a subscription on the machine fills in lanes there", where)
		}
		if slices.Contains(labels, p.FillIn.RunsOn) {
			configError("%s: %s is a subscription's own runner; a fill-in has runners of its own",
				where, p.FillIn.RunsOn)
		}
		labels = append(labels, p.FillIn.RunsOn)
		for _, other := range p.FillIn.Of {
			if !providers.Has(other) || other == p.ID {
				configError("%s: %s is not another subscription", where, py.Repr(other))
			}
			if slices.Contains(filled, other) {
				configError("%s: another subscription fills %s's lanes already", where, other)
			}
			filled = append(filled, other)
		}
	}
	var priority []string
	var listed any = providers
	if obj.Truthy("priority") {
		listed = obj.Get("priority")
	}
	for _, p := range py.MustIter(listed) {
		priority = append(priority, py.Str(p))
	}
	missing, extra := []string{}, []string{}
	for _, p := range providers.Keys() {
		if !slices.Contains(priority, p) {
			missing = append(missing, p)
		}
	}
	for _, p := range priority {
		if !providers.Has(p) {
			extra = append(extra, p)
		}
	}
	if len(missing) > 0 || len(extra) > 0 || len(repeated(priority)) > 0 {
		configError("%s: priority must name every provider once (missing %s, unknown %s)",
			ProvidersPath, py.Repr(missing), py.Repr(extra))
	}
	maxParallel := py.MustInt(obj.GetOr("max_parallel", 1))
	if maxParallel < 1 {
		configError("%s: max_parallel must be at least 1", ProvidersPath)
	}
	tiers := tiersOf(obj.Get("tiers"))
	machine := py.MustInt(obj.GetOr("machine_parallel", maxParallel))
	if machine < 0 {
		configError("%s: machine_parallel must be 0 or more", ProvidersPath)
	}
	planLanes := py.MustInt(obj.GetOr("plan_lanes", 0))
	if planLanes < 0 {
		configError("%s: plan_lanes must be 0 or more", ProvidersPath)
	}
	for _, p := range all {
		for _, extra := range p.ExtraModels {
			if extra.TakesOver != "" && (!providers.Has(extra.TakesOver) || extra.TakesOver == p.ID) {
				configError("%s: %s's %s takes over %s, which is not another provider",
					ProvidersPath, p.ID, extra.Model, py.Repr(extra.TakesOver))
			}
		}
	}
	balance, ok := obj.GetOr("balance", false).(bool)
	if !ok {
		configError("%s: balance must be true or false", ProvidersPath)
	}
	pool = &Pool{MaxParallel: maxParallel, Priority: priority, Providers: providers,
		MachineParallel: machine, PlanLanes: planLanes, Tiers: tiers, Balance: balance}
	// Every model a provider runs has its place in its own tier's order, so the router always
	// knows which to try first, and a model never has two tiers.
	for _, p := range all {
		for _, seat := range pool.Seats(p) {
			var placed []string
			for _, name := range tiers.Keys() {
				for _, e := range tiers.Get(name).([]*TierEntry) {
					if e.Model == seat.Model {
						placed = append(placed, name)
						break
					}
				}
			}
			if len(placed) != 1 || placed[0] != seat.Tier {
				why := fmt.Sprintf("tiers.%s does not list it", seat.Tier)
				if len(placed) > 0 {
					why = "tiers lists it under " + strings.Join(placed, ", ")
				}
				configError("%s: %s runs %s as %s, but %s", ProvidersPath, p.ID, seat.Model, seat.Tier, why)
			}
		}
	}
	return pool
}

// Load is load.
func Load(root string) *Pool {
	path := purePath(root, ProvidersPath)
	data, err := os.ReadFile(path)
	if errors.Is(err, fs.ErrNotExist) {
		configError("%s does not exist", path)
	}
	herr.Must(err)
	// read_text's UnicodeDecodeError is a ValueError, which the Python reports as bad JSON. The
	// text after "is not valid JSON: " is Go's, not Python's json module's.
	if !utf8.Valid(data) {
		configError("%s is not valid JSON: 'utf-8' codec can't decode it", path)
	}
	raw, err := pyjson.Loads(data)
	if err != nil {
		configError("%s is not valid JSON: %s", path, err.Error())
	}
	return Parse(raw)
}
