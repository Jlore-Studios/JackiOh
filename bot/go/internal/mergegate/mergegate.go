// Package mergegate is what a bot pull request's diff must not hold to merge itself (#2986,
// section 1; the owner's decision of 2026-11-04, amended the same day): the paths that merge
// themselves only on a condition, read from the files themselves, with no model.
//
//   - A migration (`migration_paths`): it merges itself unless it loses data (an added line that
//     drops a table, schema or column, runs `TRUNCATE` or `DELETE FROM`, or rewrites every row of a
//     table, an `UPDATE … SET` with no `WHERE`, #3009) or changes who may read
//     or write what (a policy created, altered or dropped, row-level security turned on, off or
//     forced, a grant or a revoke, a `SECURITY DEFINER` function, the `auth` or `storage` schemas),
//     or edits or deletes a migration `main` already has (`Migration`; the caller reads the status).
//   - A lock (`lock_files`): it merges itself unless it gains a package name `main`'s lock lacks, a
//     new dependency, direct or transitive (`LockPackages`). A Cargo manifest it changes must name
//     only packages its lock holds (`CargoDependencies`): CI builds with no `--locked`, so a
//     dependency added to a manifest and not to the lock would never meet the lock's check.
//
// Every rule errs toward a person: what it cannot read, or a statement it does not know to be
// safe, goes to one.
package mergegate

import (
	"path"
	"regexp"
	"strings"
)

// LossKind and AccessKind are a migration's `Finding.Kind`.
const (
	LossKind   = "loses data"
	AccessKind = "changes who may read or write what"
)

// StatementChars is how much of a statement a finding quotes.
const StatementChars = 160

// Finding is the first statement of a migration that keeps it with a person, and why.
type Finding struct {
	Kind      string // LossKind or AccessKind
	Statement string // the statement, its whitespace collapsed, cut to StatementChars
}

// Migration is the first statement of `sql`, a migration's whole text, that loses data or changes
// access, or nil for none. Comments (`--` to the end of the line, `/* … */`, and `COMMENT ON`, whose
// text is prose) are not read; string literals are, and a dollar-quoted body (`$$ … $$`, a
// function's or a `DO` block's) is read as statements of its own, since it runs too.
func Migration(sql string) *Finding {
	for _, s := range statements(sql) {
		upper := strings.ToUpper(s.text)
		if strings.HasPrefix(upper, "COMMENT ") {
			continue
		}
		kind := ""
		switch {
		case losesData(upper):
			kind = LossKind
		case changesAccess(upper):
			kind = AccessKind
		}
		if kind != "" {
			shown := s.text
			if runes := []rune(shown); len(runes) > StatementChars {
				shown = string(runes[:StatementChars]) + "…"
			}
			return &Finding{Kind: kind, Statement: shown}
		}
		for _, body := range s.bodies {
			if found := Migration(body); found != nil {
				return found
			}
		}
	}
	return nil
}

var (
	truncateRe    = regexp.MustCompile(`\bTRUNCATE\b`)
	updateSetRe   = regexp.MustCompile(`\bUPDATE\s+(?:ONLY\s+)?([^\s(]+)(?:\s+(?:AS\s+)?([^\s(]+))?\s+SET\b`)
	deleteFromRe  = regexp.MustCompile(`\bDELETE\s+FROM\b`)
	dropColumnRe  = regexp.MustCompile(`\bDROP\s+COLUMN\b`)
	dropRe        = regexp.MustCompile(`\bDROP\s+(\S+)(?:\s+(\S+))?`)
	alterTableRe  = regexp.MustCompile(`^ALTER\s+TABLE\b`)
	policyRe      = regexp.MustCompile(`\b(?:CREATE|ALTER|DROP)\s+POLICY\b`)
	rowSecurityRe = regexp.MustCompile(`\b(?:ENABLE|DISABLE|FORCE)\s+ROW\s+LEVEL\s+SECURITY\b`)
	grantRe       = regexp.MustCompile(`\b(?:GRANT|REVOKE)\b|\bALTER\s+DEFAULT\s+PRIVILEGES\b`)
	definerRe     = regexp.MustCompile(`\bSECURITY\s+DEFINER\b`)
	schemaRe      = regexp.MustCompile(`(?:\b|")(?:AUTH|STORAGE)"?\s*\.|\bSCHEMA\s+(?:IF\s+(?:NOT\s+)?EXISTS\s+)?"?(?:AUTH|STORAGE)\b|\bSEARCH_PATH\b[^;]*\b(?:AUTH|STORAGE)\b`)
)

// SafeDrops are what a statement that starts with DROP may drop without losing data (the usual
// drop-then-recreate): a function, a trigger, an index or a view, with or without `IF EXISTS`. A
// policy's drop changes access (`changesAccess`). Any other kind (a table, a schema, a type or a
// sequence, which can take columns or values with them) loses data, and so does a `CASCADE`.
var SafeDrops = []string{"FUNCTION", "TRIGGER", "INDEX", "VIEW", "POLICY"}

// SafeAlterDrops are what `ALTER TABLE … DROP` may drop without losing data: a constraint, and a
// column's default or `NOT NULL` (`ALTER COLUMN … DROP DEFAULT`). Anything else it drops is a
// column (`DROP [COLUMN] [IF EXISTS] <name>`).
var SafeAlterDrops = []string{"CONSTRAINT", "DEFAULT", "NOT"}

// losesData is: the upper-cased statement drops a table, schema or column, deletes or truncates
// rows, or rewrites every row of a table (`Migration`'s rules).
func losesData(upper string) bool {
	if truncateRe.MatchString(upper) || deleteFromRe.MatchString(upper) || dropColumnRe.MatchString(upper) ||
		updatesEveryRow(upper) {
		return true
	}
	alter := alterTableRe.MatchString(upper)
	if strings.HasPrefix(upper, "DROP ") && strings.Contains(" "+upper+" ", " CASCADE ") {
		return true // it drops whatever depends on it, a column of a type or a table of a view among them
	}
	for _, m := range dropRe.FindAllStringSubmatch(upper, -1) {
		what := strings.Trim(m[1], `"(,`)
		if alter {
			if !contains(SafeAlterDrops, what) || (what == "NOT" && m[2] != "NULL") {
				return true
			}
			continue
		}
		if !contains(SafeDrops, what) {
			return true
		}
	}
	return false
}

// updatesEveryRow is: the upper-cased statement holds an `UPDATE … SET` with no `WHERE` of its own
// (#3009), `UPDATE … FROM` among them, which rewrites every row of its table. Its `WHERE` is the
// first one after its `SET` at its own depth of parentheses and outside a string or a quoted name: a
// subquery's (`SET x = (SELECT … WHERE …)`) is not, and one inside a `WITH` ends at the parenthesis
// that closes it. An `INSERT … ON CONFLICT DO UPDATE SET` names no table, and changes only the rows
// that conflict. An `UPDATE` written inside a string is read too: it errs toward a person.
func updatesEveryRow(upper string) bool {
	runes := []rune(upper)
	for _, at := range updateSetRe.FindAllStringSubmatchIndex(upper, -1) {
		if upper[at[2]:at[3]] == "SET" {
			continue // `DO UPDATE SET …`: no table of its own
		}
		start := len([]rune(upper[:at[0]]))
		depth, quote := 0, rune(0)
		for _, c := range runes[:start] {
			depth, quote = sqlStep(c, depth, quote)
		}
		if !hasWhere(runes[len([]rune(upper[:at[1]])):], depth, quote) {
			return true
		}
	}
	return false
}

// sqlStep is the parenthesis depth and the open quote (`'` or `"`, 0 for none) after `c`: a doubled
// quote closes and opens again, which leaves it open.
func sqlStep(c rune, depth int, quote rune) (int, rune) {
	switch {
	case quote != 0:
		if c == quote {
			quote = 0
		}
	case c == '\'' || c == '"':
		quote = c
	case c == '(':
		depth++
	case c == ')':
		depth--
	}
	return depth, quote
}

// hasWhere is: `rest`, what follows an `UPDATE`'s `SET` at parenthesis depth `depth` (in a quote
// when `quote` is not 0), holds a `WHERE` word at that depth outside a quote before it ends (the
// statement's end, or a parenthesis that closes below `depth`).
func hasWhere(rest []rune, depth int, quote rune) bool {
	level := depth
	for i, c := range rest {
		if quote == 0 && level == depth && c == 'W' && (i == 0 || !sqlWord(rest[i-1])) && i+5 <= len(rest) &&
			string(rest[i:i+5]) == "WHERE" && (i+5 == len(rest) || !sqlWord(rest[i+5])) {
			return true
		}
		level, quote = sqlStep(c, level, quote)
		if level < depth {
			return false
		}
	}
	return false
}

// sqlWord is: `c` may be part of a name (a letter, a digit, `_` or `$`).
func sqlWord(c rune) bool {
	return c == '_' || c == '$' || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9')
}

// changesAccess is: the upper-cased statement changes who may read or write what.
func changesAccess(upper string) bool {
	return policyRe.MatchString(upper) || rowSecurityRe.MatchString(upper) || grantRe.MatchString(upper) ||
		definerRe.MatchString(upper) || schemaRe.MatchString(upper)
}

func contains(list []string, s string) bool {
	for _, x := range list {
		if x == s {
			return true
		}
	}
	return false
}

// statement is one statement of a migration: its text, comments out and whitespace collapsed, each
// dollar-quoted string in it written `$$`, and those strings' contents (`bodies`).
type statement struct {
	text   string
	bodies []string
}

// statements is `sql` without its comments, split at each `;` outside a string, a quoted name or a
// dollar quote; empty ones are left out.
func statements(sql string) []statement {
	var out []statement
	var current strings.Builder
	var bodies []string
	flush := func() {
		if s := strings.Join(strings.Fields(current.String()), " "); s != "" {
			out = append(out, statement{s, bodies})
		}
		current.Reset()
		bodies = nil
	}
	runes := []rune(sql)
	for i := 0; i < len(runes); i++ {
		c := runes[i]
		next := rune(0)
		if i+1 < len(runes) {
			next = runes[i+1]
		}
		switch {
		case c == '-' && next == '-':
			for i < len(runes) && runes[i] != '\n' {
				i++
			}
			current.WriteRune(' ')
		case c == '/' && next == '*':
			depth := 0
			for ; i < len(runes); i++ {
				if runes[i] == '/' && i+1 < len(runes) && runes[i+1] == '*' {
					depth++
					i++
				} else if runes[i] == '*' && i+1 < len(runes) && runes[i+1] == '/' {
					depth--
					i++
					if depth == 0 {
						break
					}
				}
			}
			current.WriteRune(' ')
		case c == '\'' || c == '"':
			// A string or a quoted name, kept as it is: `''` (or `""`) inside is the quote itself.
			current.WriteRune(c)
			for i++; i < len(runes); i++ {
				current.WriteRune(runes[i])
				if runes[i] == c {
					if i+1 < len(runes) && runes[i+1] == c {
						i++
						current.WriteRune(c)
						continue
					}
					break
				}
			}
		case c == '$' && dollarTag(runes, i) > i:
			end := dollarTag(runes, i)
			tag := string(runes[i : end+1])
			rest := string(runes[end+1:])
			closing := strings.Index(rest, tag)
			if closing < 0 {
				closing = len(rest) // unclosed: the rest is its body
			}
			bodies = append(bodies, rest[:closing])
			current.WriteString(" $$ ")
			i = end + len([]rune(rest[:min(closing+len(tag), len(rest))]))
		case c == ';':
			flush()
		default:
			current.WriteRune(c)
		}
	}
	flush()
	return out
}

// dollarTag is the index of the closing `$` of a dollar quote's delimiter that opens at `at`
// (`$$`, `$tag$`), or `at` when none does there (`$1` is a parameter).
func dollarTag(runes []rune, at int) int {
	for j := at + 1; j < len(runes); j++ {
		r := runes[j]
		switch {
		case r == '$':
			return j
		case r == '_' || (r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z') || (j > at+1 && r >= '0' && r <= '9'):
			continue
		}
		return at
	}
	return at
}

// LockPackages is the package names a lock holds, by its file's name: a `[[package]]`'s `name` in
// a `Cargo.lock`, and each key of `packages:` in a `pnpm-lock.yaml` without its version. False for
// a lock whose format it does not read.
func LockPackages(lockPath string, text string) ([]string, bool) {
	switch path.Base(lockPath) {
	case "Cargo.lock":
		return cargoPackages(text), true
	case "pnpm-lock.yaml":
		return pnpmPackages(text), true
	}
	return nil, false
}

var tomlNameRe = regexp.MustCompile(`^name\s*=\s*"([^"]*)"`)

func cargoPackages(text string) []string {
	var out []string
	inPackage := false
	for _, raw := range strings.Split(text, "\n") {
		line := strings.TrimSpace(raw)
		if strings.HasPrefix(line, "[") {
			inPackage = line == "[[package]]"
			continue
		}
		if m := tomlNameRe.FindStringSubmatch(line); inPackage && m != nil {
			out = append(out, m[1])
			inPackage = false
		}
	}
	return out
}

var pnpmKeyRe = regexp.MustCompile(`^  ([^ #].*?):(?:\s+\{\})?\s*$`)

func pnpmPackages(text string) []string {
	var out []string
	inPackages := false
	for _, line := range strings.Split(strings.ReplaceAll(text, "\r\n", "\n"), "\n") {
		if line != "" && line[0] != ' ' && line[0] != '#' {
			inPackages = strings.TrimSpace(line) == "packages:"
			continue
		}
		m := pnpmKeyRe.FindStringSubmatch(line)
		if !inPackages || m == nil {
			continue
		}
		out = append(out, pnpmName(strings.Trim(m[1], `'"`)))
	}
	return out
}

// pnpmName is a `packages:` key's package name: `name@1.2.3`, `@scope/name@1.2.3` (lockfile 9),
// `/name@1.2.3(peer@2)` (6) or `/name/1.2.3` (5), without the version or the peers.
func pnpmName(key string) string {
	key = strings.TrimPrefix(key, "/")
	if i := strings.Index(key, "("); i > 0 {
		key = key[:i]
	}
	if i := strings.LastIndex(key, "@"); i > 0 {
		return key[:i]
	}
	if i := strings.LastIndex(key, "/"); i > 0 {
		return key[:i]
	}
	return key
}

var (
	tomlHeaderRe  = regexp.MustCompile(`^\[([^\[\]]+)\]\s*(?:#.*)?$`)
	tomlKeyRe     = regexp.MustCompile(`^("[^"]+"|'[^']+'|[A-Za-z0-9_-]+)(?:\.[A-Za-z0-9_-]+)*\s*=`)
	tomlPackageRe = regexp.MustCompile(`\bpackage\s*=\s*"([^"]+)"`)
)

// CargoDependencies is the package names a Cargo manifest depends on: every key of a dependency
// table (`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, `[workspace.dependencies]`,
// a `[target.….dependencies]`), or the table a `[dependencies.<name>]` names, each read as the
// `package = "…"` it renames when it has one.
func CargoDependencies(manifest string) []string {
	var out []string
	table := false   // in a dependency table: each key is a dependency
	single := ""     // in `[dependencies.<name>]`: that one
	renamed := false // the single one's `package =` was read
	seen := map[string]bool{}
	add := func(name string) {
		name = strings.Trim(name, `"'`)
		if name != "" && !seen[name] {
			seen[name] = true
			out = append(out, name)
		}
	}
	end := func() {
		if single != "" && !renamed {
			add(single)
		}
		single, renamed = "", false
	}
	for _, raw := range strings.Split(manifest, "\n") {
		line := strings.TrimSpace(raw)
		if strings.HasPrefix(line, "[[") {
			end() // an array of tables (`[[bin]]`): no dependency's
			table = false
			continue
		}
		if m := tomlHeaderRe.FindStringSubmatch(line); m != nil {
			end()
			header := strings.TrimSpace(m[1])
			table = isDependencyTable(header)
			if !table {
				for _, kind := range dependencyTables {
					if before, name, ok := strings.Cut(header, kind+"."); ok && (before == "" || strings.HasSuffix(before, ".")) {
						single = name
					}
				}
			}
			continue
		}
		if single != "" {
			if m := tomlPackageRe.FindStringSubmatch(line); m != nil && strings.HasPrefix(line, "package") {
				add(m[1])
				renamed = true
			}
			continue
		}
		if !table {
			continue
		}
		m := tomlKeyRe.FindStringSubmatch(line)
		if m == nil {
			continue
		}
		if p := tomlPackageRe.FindStringSubmatch(line[len(m[0]):]); p != nil {
			add(p[1])
		} else {
			add(m[1])
		}
	}
	end()
	return out
}

// dependencyTables are the names of Cargo's dependency tables.
var dependencyTables = []string{"dependencies", "dev-dependencies", "build-dependencies"}

func isDependencyTable(header string) bool {
	for _, kind := range dependencyTables {
		if header == kind || strings.HasSuffix(header, "."+kind) {
			return true
		}
	}
	return false
}
