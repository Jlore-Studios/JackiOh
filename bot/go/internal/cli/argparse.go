package cli

// The command line's parser: argparse (Python 3.13) as far as `parser()` uses it, so a person or a
// workflow sees the same thing: `-h`/`--help` prints the help and exits 0; a missing command or
// required argument, a bad choice, a value that is not an int, an option that wants a value and
// has none, an ambiguous abbreviation or an argument nobody takes prints the usage and
// `<prog>: error: …` to stderr and exits 2. Options may be abbreviated to any unique prefix, take
// their value as `--flag value` or `--flag=value`, and `--` ends them. The help is formatted as
// argparse's HelpFormatter formats it off a terminal (80 columns, so 78 wide), with textwrap's
// rules for where a line may break.

import (
	"fmt"
	"regexp"
	"strings"
	"unicode"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// helpWidth is HelpFormatter's width: `shutil.get_terminal_size()` falls back to 80 columns when
// no terminal (and no COLUMNS) says otherwise, less 2.
const helpWidth = 78

type actionKind int

const (
	kindHelp actionKind = iota
	kindStoreTrue
	kindStore
	kindSubparsers
	kindChoice // a subparser's line in its parent's help (`_ChoicesPseudoAction`)
)

// The nargs the harness's arguments have.
const (
	nargsOne    = ""     // None: one value
	nargsZero   = "0"    // store_true and help
	nargsAny    = "*"    // ZERO_OR_MORE
	nargsParser = "A..." // PARSER: a subcommand and everything after it
)

// argAction is argparse.Action.
type argAction struct {
	dest     string
	flags    []string // option strings; none for a positional
	kind     actionKind
	nargs    string
	isInt    bool // type=int
	required bool
	def      any
	choices  []string
	help     string
	metavar  string
	subs     []*subParser // a subparsers action's parsers, in the order they were added
}

type subParser struct {
	name   string
	help   string
	parser *ArgumentParser
}

// ArgumentParser is argparse.ArgumentParser, as far as the harness uses it.
type ArgumentParser struct {
	Prog        string
	Description string
	actions     []*argAction
	optionOrder []string // `_option_string_actions`, in insertion order
	options     map[string]*argAction
}

// newParser is ArgumentParser(prog=prog, description=description), with its `-h, --help`.
func newParser(prog, description string) *ArgumentParser {
	p := &ArgumentParser{Prog: prog, Description: description, options: map[string]*argAction{}}
	p.add(&argAction{dest: "help", flags: []string{"-h", "--help"}, kind: kindHelp, nargs: nargsZero,
		help: "show this help message and exit"})
	return p
}

func (p *ArgumentParser) add(a *argAction) *argAction {
	p.actions = append(p.actions, a)
	for _, f := range a.flags {
		p.optionOrder = append(p.optionOrder, f)
		p.options[f] = a
	}
	return a
}

// destOf is the dest argparse gives an option: its first long flag, dashes to underscores.
func destOf(flag string) string {
	return strings.ReplaceAll(strings.TrimLeft(flag, "-"), "-", "_")
}

// flag is add_argument(flag, action="store_true", help=help).
func (p *ArgumentParser) flag(flag, help string) *argAction {
	return p.add(&argAction{dest: destOf(flag), flags: []string{flag}, kind: kindStoreTrue,
		nargs: nargsZero, def: false, help: help})
}

// option is add_argument(flag, default=def, help=help): one value, a string.
func (p *ArgumentParser) option(flag string, def any, help string) *argAction {
	return p.add(&argAction{dest: destOf(flag), flags: []string{flag}, kind: kindStore,
		nargs: nargsOne, def: def, help: help})
}

// positional is add_argument(dest[, nargs="*"]).
func (p *ArgumentParser) positional(dest, nargs string) *argAction {
	a := &argAction{dest: dest, kind: kindStore, nargs: nargs}
	a.required = nargs == nargsOne
	return p.add(a)
}

// subparsers is add_subparsers(dest=dest, required=required).
func (p *ArgumentParser) subparsers(dest string, required bool) *argAction {
	return p.add(&argAction{dest: dest, kind: kindSubparsers, nargs: nargsParser, required: required})
}

// addParser is add_parser(name, help=help): its prog is the parent's and the name.
func (a *argAction) addParser(parent *ArgumentParser, name, help string) *ArgumentParser {
	sub := newParser(parent.Prog+" "+name, "")
	a.subs = append(a.subs, &subParser{name: name, help: help, parser: sub})
	a.choices = append(a.choices, name)
	return sub
}

// ---------------------------------------------------------------------------- parsing

// exitError is SystemExit from parse_args: the help (code 0, on stdout) or an error (code 2, the
// usage and the message on stderr).
type exitError struct {
	code   int
	stdout string
	stderr string
}

// unrecognized is `_UNRECOGNIZED_ARGS_ATTR`: what a subparser left, for the top parser to report.
const unrecognized = "\x00unrecognized"

// parse is parse_args: the namespace, or the exit argparse would have taken.
func (p *ArgumentParser) parse(argv []string) (ns map[string]any, exit *exitError) {
	defer func() {
		if r := recover(); r != nil {
			if e, ok := r.(*exitError); ok {
				ns, exit = nil, e
				return
			}
			panic(r)
		}
	}()
	ns = map[string]any{}
	extras := p.parseKnownArgs(argv, ns)
	if len(extras) > 0 {
		p.fail("unrecognized arguments: " + strings.Join(extras, " "))
	}
	return ns, nil
}

// fail is error(): the usage and the message on stderr, exit 2.
func (p *ArgumentParser) fail(message string) {
	panic(&exitError{code: 2, stderr: p.formatUsage() + p.Prog + ": error: " + message + "\n"})
}

func actionName(a *argAction) string {
	if len(a.flags) > 0 {
		return strings.Join(a.flags, "/")
	}
	if a.metavar != "" {
		return a.metavar
	}
	return a.dest
}

// argumentError is str(ArgumentError(action, message)).
func argumentError(a *argAction, message string) string {
	if a == nil {
		return message
	}
	return "argument " + actionName(a) + ": " + message
}

// parseKnownArgs is parse_known_args (`_parse_known_args2`): defaults first, then the arguments.
func (p *ArgumentParser) parseKnownArgs(args []string, ns map[string]any) []string {
	for _, a := range p.actions {
		if a.kind == kindHelp {
			continue // dest=SUPPRESS
		}
		if _, ok := ns[a.dest]; !ok {
			ns[a.dest] = a.def
		}
	}
	extras := p.parseKnown(args, ns)
	if left, ok := ns[unrecognized]; ok {
		extras = append(extras, left.([]string)...)
		delete(ns, unrecognized)
	}
	return extras
}

// optTuple is one of `_parse_optional`'s (action, option_string, sep, explicit_arg).
type optTuple struct {
	action      *argAction
	option      string
	sep         *string
	explicitArg *string
}

func strp(s string) *string { return &s }

var negativeNumber = regexp.MustCompile(`^-\d+$|^-\d*\.\d+$`)

// parseOptional is _parse_optional: nil for a positional, else what it may be.
func (p *ArgumentParser) parseOptional(arg string) []optTuple {
	if arg == "" || arg[0] != '-' {
		return nil
	}
	if a, ok := p.options[arg]; ok {
		return []optTuple{{a, arg, nil, nil}}
	}
	if py.Len(arg) == 1 {
		return nil
	}
	if option, explicit, found := strings.Cut(arg, "="); found {
		if a, ok := p.options[option]; ok {
			return []optTuple{{a, option, strp("="), strp(explicit)}}
		}
	}
	if tuples := p.optionTuples(arg); len(tuples) > 0 {
		return tuples
	}
	if negativeNumber.MatchString(arg) {
		return nil // no option of the harness's looks like a negative number
	}
	if strings.Contains(arg, " ") {
		return nil
	}
	return []optTuple{{nil, arg, nil, nil}}
}

// optionTuples is _get_option_tuples: the options `arg` abbreviates.
func (p *ArgumentParser) optionTuples(arg string) []optTuple {
	var out []optTuple
	prefix, explicit, found := strings.Cut(arg, "=")
	var sep, exp *string
	if found {
		sep, exp = strp("="), strp(explicit)
	}
	if arg[1] == '-' {
		for _, option := range p.optionOrder {
			if strings.HasPrefix(option, prefix) {
				out = append(out, optTuple{p.options[option], option, sep, exp})
			}
		}
		return out
	}
	short := arg[:2]
	shortExplicit := arg[2:]
	for _, option := range p.optionOrder {
		if option == short {
			out = append(out, optTuple{p.options[option], option, strp(""), strp(shortExplicit)})
		} else if strings.HasPrefix(option, prefix) {
			out = append(out, optTuple{p.options[option], option, sep, exp})
		}
	}
	return out
}

// nargsPattern is _get_nargs_pattern.
func nargsPattern(a *argAction) string {
	if len(a.flags) > 0 {
		switch a.nargs {
		case nargsOne:
			return "([A])"
		case nargsZero:
			return "()"
		}
		panic("cli: no option takes nargs " + a.nargs)
	}
	switch a.nargs {
	case nargsOne:
		return "(-*A-*)"
	case nargsAny:
		return "(-*[A-]*)"
	case nargsParser:
		return "(-*A[-AO]*)"
	}
	panic("cli: no positional takes nargs " + a.nargs)
}

// matchArgument is _match_argument.
func (p *ArgumentParser) matchArgument(a *argAction, pattern string) int {
	m := regexp.MustCompile("^" + nargsPattern(a)).FindStringSubmatch(pattern)
	if m == nil {
		p.fail(argumentError(a, "expected one argument"))
	}
	return len(m[1])
}

// matchPartial is _match_arguments_partial.
func matchPartial(actions []*argAction, pattern string) []int {
	for i := len(actions); i > 0; i-- {
		var pat strings.Builder
		for _, a := range actions[:i] {
			pat.WriteString(nargsPattern(a))
		}
		re := regexp.MustCompile("^" + pat.String())
		loc := re.FindStringSubmatchIndex(pattern)
		if loc == nil {
			continue
		}
		var result []int
		for g := 1; g <= i; g++ {
			result = append(result, loc[2*g+1]-loc[2*g])
		}
		if end := loc[1]; end < len(pattern) && pattern[end] == 'O' {
			for len(result) > 0 && result[len(result)-1] == 0 {
				result = result[:len(result)-1]
			}
		}
		return result
	}
	return nil
}

func removeFirst(xs []string, s string) []string {
	for i, x := range xs {
		if x == s {
			return append(xs[:i:i], xs[i+1:]...)
		}
	}
	return xs
}

// parseKnown is _parse_known_args.
func (p *ArgumentParser) parseKnown(argStrings []string, ns map[string]any) []string {
	optionIndices := map[int][]optTuple{}
	var patternParts []byte
	for i := 0; i < len(argStrings); i++ {
		if argStrings[i] == "--" {
			patternParts = append(patternParts, '-')
			for i++; i < len(argStrings); i++ {
				patternParts = append(patternParts, 'A')
			}
			break
		}
		if tuples := p.parseOptional(argStrings[i]); tuples == nil {
			patternParts = append(patternParts, 'A')
		} else {
			optionIndices[i] = tuples
			patternParts = append(patternParts, 'O')
		}
	}
	pattern := string(patternParts)

	seen := map[*argAction]bool{}
	var extras []string

	takeAction := func(a *argAction, args []string) {
		seen[a] = true
		values := p.getValues(a, args)
		p.call(a, ns, values)
	}

	consumeOptional := func(start int) int {
		tuples := optionIndices[start]
		if len(tuples) > 1 {
			var options []string
			for _, t := range tuples {
				options = append(options, t.option)
			}
			p.fail(argumentError(nil, fmt.Sprintf("ambiguous option: %s could match %s", argStrings[start],
				strings.Join(options, ", "))))
		}
		t := tuples[0]
		a, option, sep, explicit := t.action, t.option, t.sep, t.explicitArg
		type taken struct {
			a    *argAction
			args []string
		}
		var actionTuples []taken
		var stop int
		for {
			if a == nil {
				extras = append(extras, argStrings[start])
				return start + 1
			}
			if explicit != nil {
				argCount := p.matchArgument(a, "A")
				if argCount == 0 && option[1] != '-' && *explicit != "" {
					if (sep != nil && *sep != "") || (*explicit)[0] == '-' {
						p.fail(argumentError(a, "ignored explicit argument "+py.Repr(*explicit)))
					}
					actionTuples = append(actionTuples, taken{a, nil})
					char := option[:1]
					option = char + (*explicit)[:1]
					if next, ok := p.options[option]; ok {
						a = next
						rest := (*explicit)[1:]
						switch {
						case rest == "":
							sep, explicit = nil, nil
						case rest[0] == '=':
							sep, explicit = strp("="), strp(rest[1:])
						default:
							sep, explicit = strp(""), strp(rest)
						}
					} else {
						extras = append(extras, char+*explicit)
						stop = start + 1
						break
					}
				} else if argCount == 1 {
					stop = start + 1
					actionTuples = append(actionTuples, taken{a, []string{*explicit}})
					break
				} else {
					p.fail(argumentError(a, "ignored explicit argument "+py.Repr(*explicit)))
				}
			} else {
				first := start + 1
				argCount := p.matchArgument(a, pattern[first:])
				stop = first + argCount
				actionTuples = append(actionTuples, taken{a, append([]string(nil), argStrings[first:stop]...)})
				break
			}
		}
		for _, t := range actionTuples {
			takeAction(t.a, t.args)
		}
		return stop
	}

	var positionals []*argAction
	for _, a := range p.actions {
		if len(a.flags) == 0 {
			positionals = append(positionals, a)
		}
	}

	consumePositionals := func(start int) int {
		counts := matchPartial(positionals, pattern[start:])
		for i, n := range counts {
			a := positionals[i]
			args := append([]string(nil), argStrings[start:start+n]...)
			if a.nargs == nargsParser {
				if pattern[start] == '-' {
					args = removeFirst(args, "--")
				}
			} else if strings.Contains(pattern[start:start+n], "-") {
				args = removeFirst(args, "--")
			}
			start += n
			takeAction(a, args)
		}
		positionals = positionals[len(counts):]
		return start
	}

	start := 0
	maxOption := -1
	for i := range optionIndices {
		if i > maxOption {
			maxOption = i
		}
	}
	for start <= maxOption {
		next := start
		for next <= maxOption {
			if _, ok := optionIndices[next]; ok {
				break
			}
			next++
		}
		if start != next {
			end := consumePositionals(start)
			if end > start {
				start = end
				continue
			}
			start = end
		}
		if _, ok := optionIndices[start]; !ok {
			extras = append(extras, argStrings[start:next]...)
			start = next
		}
		start = consumeOptional(start)
	}
	stop := consumePositionals(start)
	extras = append(extras, argStrings[stop:]...)

	var required []string
	for _, a := range p.actions {
		if !seen[a] && a.required {
			required = append(required, actionName(a))
		}
	}
	if len(required) > 0 {
		p.fail(argumentError(nil, "the following arguments are required: "+strings.Join(required, ", ")))
	}
	return extras
}

// getValues is _get_values.
func (p *ArgumentParser) getValues(a *argAction, args []string) any {
	switch {
	case len(args) == 0 && a.nargs == nargsAny && len(a.flags) == 0:
		return []string{}
	case len(args) == 1 && a.nargs == nargsOne:
		v := p.getValue(a, args[0])
		p.checkValue(a, v)
		return v
	case a.nargs == nargsParser:
		p.checkValue(a, args[0])
		return args
	}
	return args
}

// getValue is _get_value: the string, or with type=int its int().
func (p *ArgumentParser) getValue(a *argAction, s string) any {
	if !a.isInt {
		return s
	}
	n, err := py.Int(s)
	if err != nil {
		p.fail(argumentError(a, "invalid int value: "+py.Repr(s)))
	}
	return n
}

// checkValue is _check_value.
func (p *ArgumentParser) checkValue(a *argAction, v any) {
	if a.choices == nil {
		return
	}
	s := py.Str(v)
	for _, c := range a.choices {
		if c == s {
			return
		}
	}
	quoted := make([]string, len(a.choices))
	for i, c := range a.choices {
		quoted[i] = py.Repr(c)
	}
	p.fail(argumentError(a, fmt.Sprintf("invalid choice: %s (choose from %s)", py.Repr(s), strings.Join(quoted, ", "))))
}

// call is Action.__call__.
func (p *ArgumentParser) call(a *argAction, ns map[string]any, values any) {
	switch a.kind {
	case kindHelp:
		panic(&exitError{code: 0, stdout: p.formatHelp()})
	case kindStoreTrue:
		ns[a.dest] = true
	case kindStore:
		ns[a.dest] = values
	case kindSubparsers:
		args := values.([]string)
		name := args[0]
		ns[a.dest] = name
		var sub *ArgumentParser
		for _, s := range a.subs {
			if s.name == name {
				sub = s.parser
			}
		}
		subNS := map[string]any{}
		left := sub.parseKnownArgs(args[1:], subNS)
		for k, v := range subNS {
			ns[k] = v
		}
		if len(left) > 0 {
			had, _ := ns[unrecognized].([]string)
			ns[unrecognized] = append(had, left...)
		}
	}
}

// ---------------------------------------------------------------------------- help

// cleanHelp is the end of HelpFormatter.format_help: runs of blank lines to one, and one newline
// at the end.
func cleanHelp(text string) string {
	for strings.Contains(text, "\n\n\n") {
		text = strings.ReplaceAll(text, "\n\n\n", "\n\n")
	}
	return strings.Trim(text, "\n") + "\n"
}

// formatUsage is format_usage.
func (p *ArgumentParser) formatUsage() string {
	return cleanHelp("usage: " + p.usage() + "\n\n")
}

// metavar is _metavar_formatter's one name.
func metavar(a *argAction, def string) string {
	if a.metavar != "" {
		return a.metavar
	}
	if a.choices != nil {
		return "{" + strings.Join(a.choices, ",") + "}"
	}
	return def
}

// formatArgs is _format_args.
func formatArgs(a *argAction, def string) string {
	m := metavar(a, def)
	switch a.nargs {
	case nargsAny:
		return "[" + m + " ...]"
	case nargsParser:
		return m + " ..."
	}
	return m
}

// usagePart is one action's part of the usage (`_get_actions_usage_parts`).
func usagePart(a *argAction) string {
	if len(a.flags) == 0 {
		return formatArgs(a, a.dest)
	}
	part := a.flags[0]
	if a.nargs != nargsZero {
		part += " " + formatArgs(a, strings.ToUpper(a.dest))
	}
	if !a.required {
		part = "[" + part + "]"
	}
	return part
}

// usage is _format_usage after its prefix, wrapped as argparse wraps it.
func (p *ArgumentParser) usage() string {
	const prefix = "usage: "
	var optionals, positionals []*argAction
	for _, a := range p.actions {
		if len(a.flags) > 0 {
			optionals = append(optionals, a)
		} else {
			positionals = append(positionals, a)
		}
	}
	parts := func(actions []*argAction) []string {
		var out []string
		for _, a := range actions {
			out = append(out, usagePart(a))
		}
		return out
	}
	optParts, posParts := parts(optionals), parts(positionals)
	all := strings.Join(append(append([]string(nil), optParts...), posParts...), " ")
	usage := p.Prog
	if all != "" {
		usage += " " + all
	}
	textWidth := helpWidth
	if py.Len(prefix)+py.Len(usage) <= textWidth {
		return usage
	}
	getLines := func(parts []string, indent string, withPrefix bool) []string {
		var lines, line []string
		lineLen := py.Len(indent) - 1
		if withPrefix {
			lineLen = py.Len(prefix) - 1
		}
		for _, part := range parts {
			if lineLen+1+py.Len(part) > textWidth && len(line) > 0 {
				lines = append(lines, indent+strings.Join(line, " "))
				line = nil
				lineLen = py.Len(indent) - 1
			}
			line = append(line, part)
			lineLen += py.Len(part) + 1
		}
		if len(line) > 0 {
			lines = append(lines, indent+strings.Join(line, " "))
		}
		if withPrefix {
			lines[0] = py.SliceFrom(lines[0], py.Len(indent))
		}
		return lines
	}
	var lines []string
	if float64(py.Len(prefix)+py.Len(p.Prog)) <= 0.75*float64(textWidth) {
		indent := strings.Repeat(" ", py.Len(prefix)+py.Len(p.Prog)+1)
		switch {
		case len(optParts) > 0:
			lines = getLines(append([]string{p.Prog}, optParts...), indent, true)
			lines = append(lines, getLines(posParts, indent, false)...)
		case len(posParts) > 0:
			lines = getLines(append([]string{p.Prog}, posParts...), indent, true)
		default:
			lines = []string{p.Prog}
		}
	} else {
		indent := strings.Repeat(" ", py.Len(prefix))
		lines = getLines(append(append([]string(nil), optParts...), posParts...), indent, false)
		if len(lines) > 1 {
			lines = append(getLines(optParts, indent, false), getLines(posParts, indent, false)...)
		}
		lines = append([]string{p.Prog}, lines...)
	}
	return strings.Join(lines, "\n")
}

// invocation is _format_action_invocation.
func invocation(a *argAction) string {
	if len(a.flags) == 0 {
		return metavar(a, a.dest)
	}
	if a.nargs == nargsZero {
		return strings.Join(a.flags, ", ")
	}
	return strings.Join(a.flags, ", ") + " " + formatArgs(a, strings.ToUpper(a.dest))
}

// subactions is _iter_indented_subactions: a subparsers action's lines, one per subparser.
func subactions(a *argAction) []*argAction {
	var out []*argAction
	for _, s := range a.subs {
		out = append(out, &argAction{dest: s.name, kind: kindChoice, help: s.help, metavar: s.name})
	}
	return out
}

// formatHelp is format_help.
func (p *ArgumentParser) formatHelp() string {
	maxHelpPosition := min(24, max(helpWidth-20, 4))
	actionMaxLength := 0
	for _, a := range p.actions {
		actionMaxLength = max(actionMaxLength, py.Len(invocation(a))+2)
		for _, s := range subactions(a) {
			actionMaxLength = max(actionMaxLength, py.Len(invocation(s))+4)
		}
	}
	helpPosition := min(actionMaxLength+2, maxHelpPosition)

	var formatAction func(a *argAction, indent int) string
	formatAction = func(a *argAction, indent int) string {
		helpW := max(helpWidth-helpPosition, 11)
		actionWidth := helpPosition - indent - 2
		header := invocation(a)
		indentFirst := 0
		pad := strings.Repeat(" ", indent)
		switch {
		case a.help == "":
			header = pad + header + "\n"
		case py.Len(header) <= actionWidth:
			header = pad + py.Ljust(header, actionWidth) + "  "
		default:
			header = pad + header + "\n"
			indentFirst = helpPosition
		}
		parts := []string{header}
		if strings.TrimSpace(a.help) != "" {
			lines := splitHelpLines(a.help, helpW)
			parts = append(parts, strings.Repeat(" ", indentFirst)+lines[0]+"\n")
			for _, line := range lines[1:] {
				parts = append(parts, strings.Repeat(" ", helpPosition)+line+"\n")
			}
		} else if !strings.HasSuffix(header, "\n") {
			parts = append(parts, "\n")
		}
		for _, s := range subactions(a) {
			parts = append(parts, formatAction(s, indent+2))
		}
		return strings.Join(parts, "")
	}

	section := func(heading string, actions []*argAction) string {
		var items strings.Builder
		for _, a := range actions {
			items.WriteString(formatAction(a, 2))
		}
		if items.Len() == 0 {
			return ""
		}
		return "\n" + heading + ":\n" + items.String() + "\n"
	}

	var optionals, positionals []*argAction
	for _, a := range p.actions {
		if len(a.flags) > 0 {
			optionals = append(optionals, a)
		} else {
			positionals = append(positionals, a)
		}
	}
	text := "usage: " + p.usage() + "\n\n"
	if p.Description != "" {
		text += fillText(p.Description, max(helpWidth, 11)) + "\n\n"
	}
	text += section("positional arguments", positionals)
	text += section("options", optionals)
	return cleanHelp("\n" + text + "\n")
}

// ---------------------------------------------------------------------------- textwrap

// collapse is HelpFormatter's `_whitespace_matcher.sub(' ', text).strip()`: ASCII whitespace.
func collapse(text string) string {
	return strings.Join(strings.FieldsFunc(text, isTextWrapSpace), " ")
}

// splitHelpLines is _split_lines: textwrap.wrap of the collapsed text.
func splitHelpLines(text string, width int) []string { return wrap(collapse(text), width) }

// fillText is _fill_text with no indent: textwrap.fill of the collapsed text.
func fillText(text string, width int) string { return strings.Join(wrap(collapse(text), width), "\n") }

// isTextWrapSpace is textwrap's `_whitespace` (and re.ASCII's `\s`).
func isTextWrapSpace(r rune) bool { return strings.ContainsRune("\t\n\x0b\x0c\r ", r) }

// isLetter is `[^\d\W]`.
func isLetter(r rune) bool { return py.IsWord(r) && !unicode.IsDigit(r) }

// isWordPunct is `[\w!"\'&.,?]`.
func isWordPunct(r rune) bool { return py.IsWord(r) || strings.ContainsRune(`!"'&.,?`, r) }

// dashes is how many `-` start rs[i:].
func dashes(rs []rune, i int) int {
	n := 0
	for i+n < len(rs) && rs[i+n] == '-' {
		n++
	}
	return n
}

// emDashAt is `-{2,}(?=\w)` at rs[i:].
func emDashAt(rs []rune, i int) int {
	n := dashes(rs, i)
	if n >= 2 && i+n < len(rs) && py.IsWord(rs[i+n]) {
		return n
	}
	return 0
}

// splitChunks is TextWrapper._split with break_on_hyphens: `wordsep_re.split(text)`, without its
// empty strings. Each chunk is a run of whitespace, an em-dash between words, or a word, cut after
// a hyphen between letters.
func splitChunks(text string) []string {
	rs := []rune(text)
	var chunks []string
	at := func(i int, test func(rune) bool) bool { return i >= 0 && i < len(rs) && test(rs[i]) }
	is := func(i int, r rune) bool { return i >= 0 && i < len(rs) && rs[i] == r }
	for i := 0; i < len(rs); {
		if isTextWrapSpace(rs[i]) {
			j := i
			for j < len(rs) && isTextWrapSpace(rs[j]) {
				j++
			}
			chunks = append(chunks, string(rs[i:j]))
			i = j
			continue
		}
		if at(i-1, isWordPunct) {
			if n := emDashAt(rs, i); n > 0 {
				chunks = append(chunks, string(rs[i:i+n]))
				i += n
				continue
			}
		}
		end := len(rs)
		for j := i + 1; j <= len(rs); j++ {
			// hyphenated word: `-(?:(?<=lt{2}-)|(?<=lt-lt-))(?=lt-?lt)`
			if is(j, '-') &&
				((at(j-2, isLetter) && at(j-1, isLetter)) ||
					(at(j-3, isLetter) && is(j-2, '-') && at(j-1, isLetter))) &&
				at(j+1, isLetter) && (at(j+2, isLetter) || (is(j+2, '-') && at(j+3, isLetter))) {
				end = j + 1
				break
			}
			// end of word
			if j == len(rs) || isTextWrapSpace(rs[j]) {
				end = j
				break
			}
			// em-dash: `(?<=wp)(?=-{2,}\w)`
			if at(j-1, isWordPunct) && emDashAt(rs, j) > 0 {
				end = j
				break
			}
		}
		chunks = append(chunks, string(rs[i:end]))
		i = end
	}
	return chunks
}

// wrap is textwrap.wrap(text, width) with TextWrapper's defaults.
func wrap(text string, width int) []string {
	chunks := splitChunks(text)
	var lines []string
	blank := func(s string) bool { return strings.TrimFunc(s, unicode.IsSpace) == "" }
	for len(chunks) > 0 {
		var cur []string
		curLen := 0
		// First chunk on line is whitespace -- drop it, unless this is the very beginning of the
		// text (ie. no lines started yet).
		if blank(chunks[0]) && len(lines) > 0 {
			chunks = chunks[1:]
		}
		for len(chunks) > 0 {
			l := py.Len(chunks[0])
			if curLen+l > width {
				break
			}
			cur = append(cur, chunks[0])
			curLen += l
			chunks = chunks[1:]
		}
		// The current line is full, and the next chunk is too big to fit on *any* line
		// (_handle_long_word, with break_long_words).
		if len(chunks) > 0 && py.Len(chunks[0]) > width {
			spaceLeft := width - curLen
			if width < 1 {
				spaceLeft = 1
			}
			if spaceLeft > 0 {
				chunk := chunks[0]
				end := spaceLeft
				if py.Len(chunk) > spaceLeft {
					// break after last hyphen, but only if there are non-hyphens before it
					head := []rune(py.SliceTo(chunk, spaceLeft))
					hyphen := -1
					for k := len(head) - 1; k >= 0; k-- {
						if head[k] == '-' {
							hyphen = k
							break
						}
					}
					if hyphen > 0 && strings.Trim(string(head[:hyphen]), "-") != "" {
						end = hyphen + 1
					}
				}
				cur = append(cur, py.SliceTo(chunk, end))
				chunks[0] = py.SliceFrom(chunk, end)
			} else if len(cur) == 0 {
				cur = append(cur, chunks[0])
				chunks = chunks[1:]
			}
		}
		// If the last chunk on this line is all whitespace, drop it.
		if len(cur) > 0 && blank(cur[len(cur)-1]) {
			cur = cur[:len(cur)-1]
		}
		if len(cur) > 0 {
			lines = append(lines, strings.Join(cur, ""))
		}
	}
	return lines
}
