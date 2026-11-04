// The merge gates on a condition (#2986, section 1; the owner's decision of 2026-11-04, amended the
// same day, and #3009's `UPDATE` with no `WHERE`): a migration merges itself unless it loses data,
// changes access or edits one `main` has;
// a lock or manifest unless a lock gains a package; `build.rs`, the server's Dockerfile and the
// scripts but the two that deploy production merge themselves.
package tests

import (
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/mergegate"
)

// ------------------------------------------------------------------ the rules, read from the text

func TestMergegate_AMigrationThatLosesDataIsFound(t *testing.T) {
	for _, sql := range []string{
		"drop table public.decks;",
		"DROP TABLE IF EXISTS public.decks;",
		"drop schema app cascade;",
		"alter table public.profiles drop column allow_spectators;",
		"alter table public.profiles drop column if exists allow_spectators;",
		"ALTER TABLE public.profiles\n  DROP allow_spectators;",
		"alter table public.profiles drop if exists allow_spectators;",
		"truncate public.match_actions;",
		"delete from public.seasons where id = 'v0.3';",
		"drop type public.mode;",
		"drop function app.f() cascade;",
		// A function's body runs too.
		"create function app.purge() returns void language sql as $$ delete from public.logs $$;",
		"create function app.purge() returns void language plpgsql as $body$ begin execute 'drop table x'; end $body$;",
	} {
		found := mergegate.Migration(sql)
		if found == nil || found.Kind != mergegate.LossKind {
			t.Errorf("%q: %v, want %q", sql, found, mergegate.LossKind)
		}
	}
}

func TestMergegate_AMigrationThatChangesAccessIsFound(t *testing.T) {
	for _, sql := range []string{
		"create policy decks_read on public.decks for select using (true);",
		"drop policy if exists decks_read on public.decks;", // its drop-then-recreate counts too
		"alter policy decks_read on public.decks using (owner = auth.uid());",
		"alter table public.decks enable row level security;",
		"alter table public.decks disable row level security;",
		"alter table public.decks force row level security;",
		"grant select on public.decks to authenticated;",
		"revoke all on public.decks from anon;",
		"alter default privileges in schema app revoke execute on functions from public;",
		"create or replace function app.f() returns int language sql security definer as $$ select 1 $$;",
		"alter function app.f() security definer;",
		"create table public.x (id uuid references auth.users (id));",
		"insert into storage.buckets (id) values ('art');",
		`select "auth".uid();`,
	} {
		found := mergegate.Migration(sql)
		if found == nil || found.Kind != mergegate.AccessKind {
			t.Errorf("%q: %v, want %q", sql, found, mergegate.AccessKind)
		}
	}
}

func TestMergegate_TheUsualDropThenRecreateAndPlainChangesMergeThemselves(t *testing.T) {
	for _, sql := range []string{
		"create table public.news (id uuid primary key, body text not null);",
		"alter table public.duos_matches alter column a1 drop not null;",
		"alter table public.duos_matches alter column a1 drop default;",
		"alter table public.duos_matches drop constraint if exists duos_matches_a1_fkey;\n" +
			"alter table public.duos_matches add constraint duos_matches_a1_fkey foreign key (a1) " +
			"references public.profiles(id) on delete set null;",
		"drop function if exists app.f(int);",
		"drop trigger if exists t on public.x;",
		"drop index if exists public.season_ranks_idx;",
		"drop view if exists public.v;",
		"create temp table t (x int) on commit drop;",
		// Comments are not read, nor what a comment on a table says.
		"-- drop table public.decks; grant all to anon\n/* truncate everything */ select 1;",
		"comment on table public.x is $$Deleting from it drops nothing: delete from x; grant none.$$;",
		"comment on column public.x.y is 'drop table z';",
		"create function app.f(p int) returns int language sql as $$ select $1 + p $$;",
	} {
		if found := mergegate.Migration(sql); found != nil {
			t.Errorf("%q: %q in %q, want none", sql, found.Kind, found.Statement)
		}
	}
}

func TestMergegate_AStringOrACommentCannotHideAStatement(t *testing.T) {
	found := mergegate.Migration("select '--'; drop table public.decks;")
	assertTrue(t, found != nil && found.Kind == mergegate.LossKind, found)
	assertEqual(t, found.Statement, "drop table public.decks")
	found = mergegate.Migration("select 1 /* a /* nested */ comment */; grant select on x to anon;")
	assertTrue(t, found != nil && found.Kind == mergegate.AccessKind, found)
}

// An `UPDATE` with no `WHERE` rewrites every row of its table, so it loses data (#3009): `UPDATE …
// FROM` too, and inside a function's or a `DO` block's body. One with a `WHERE` merges itself.
func TestMergegate_AnUpdateWithNoWhereLosesData(t *testing.T) {
	for _, sql := range []string{
		"update public.profiles set allow_spectators = false;",
		"UPDATE ONLY public.profiles AS p SET rank = 0 RETURNING p.id;",
		"update public.profiles p set rank = s.rank from public.season_ranks s;",
		// A subquery's `WHERE` is not the update's, nor a `WHERE` in a string or a quoted name.
		"update public.profiles set rank = (select max(rank) from public.season_ranks where season = 'v0.5');",
		"update public.profiles set note = 'where it was', \"where\" = 1;",
		// Inside a `WITH`, its own parentheses end it.
		"with moved as (update public.decks set owner = null returning id) " +
			"select * from public.decks where id in (select id from moved);",
		"create function app.reset() returns void language sql as $$ update public.profiles set rank = 0 $$;",
		"do $$ begin if true then update public.profiles set rank = 0; end if; end $$;",
	} {
		found := mergegate.Migration(sql)
		if found == nil || found.Kind != mergegate.LossKind {
			t.Errorf("%q: %v, want %q", sql, found, mergegate.LossKind)
		}
	}
	found := mergegate.Migration("create table public.a (id int);\nupdate public.profiles\n  set rank = 0;\n")
	assertEqual(t, found.Statement, "update public.profiles set rank = 0") // the comment names it
	for _, sql := range []string{
		"update public.profiles set rank = 0 where rank is null;",
		"update public.profiles p set rank = s.rank from public.season_ranks s where s.id = p.id;",
		"update public.profiles set rank = (select 1) where id in (select id from public.season_ranks);",
		"with moved as (update public.decks set owner = null where owner = 'x' returning id) select 1;",
		"create function app.f() returns void language sql as $$ update public.a set x = 1 where id = 2 $$;",
		// Not an `UPDATE … SET` of a table: an upsert's, a trigger's, a lock's, a grant's word.
		"insert into public.ranks (id, rank) values (1, 2) on conflict (id) do update set rank = excluded.rank;",
		"create trigger t before update on public.profiles for each row execute function app.touch();",
		"select * from public.decks for update skip locked;",
		"alter table public.a add constraint a_fk foreign key (b) references public.b(id) on update cascade;",
	} {
		if found := mergegate.Migration(sql); found != nil {
			t.Errorf("%q: %q in %q, want none", sql, found.Kind, found.Statement)
		}
	}
}

func TestMergegate_TheLocksPackages(t *testing.T) {
	cargo := "version = 4\n\n[[package]]\nname = \"aho-corasick\"\nversion = \"1.1.5\"\n" +
		"dependencies = [\n \"memchr\",\n]\n\n[[package]]\nname = \"memchr\"\nversion = \"2.7.4\"\n\n" +
		"[metadata]\nname = \"not-a-package\"\n"
	names, ok := mergegate.LockPackages("Cargo.lock", cargo)
	assertTrue(t, ok)
	assertEqual(t, names, []string{"aho-corasick", "memchr"})
	pnpm := "lockfileVersion: '9.0'\n\nimporters:\n\n  .:\n    devDependencies:\n      vitest:\n" +
		"        specifier: ^3\n\npackages:\n\n  '@adobe/css-tools@4.5.0':\n    resolution: {integrity: x}\n\n" +
		"  vitest@3.2.4:\n    resolution: {integrity: y}\n\n  /old@1.0.0(peer@2.0.0):\n    resolution: {}\n\n" +
		"  /older/1.0.0: {}\n\nsnapshots:\n\n  zod@3.0.0: {}\n"
	names, ok = mergegate.LockPackages("apps/pnpm-lock.yaml", pnpm)
	assertTrue(t, ok)
	assertEqual(t, names, []string{"@adobe/css-tools", "vitest", "old", "older"})
	_, ok = mergegate.LockPackages("yarn.lock", "")
	assertFalse(t, ok)
}

func TestMergegate_ACargoManifestsDependencies(t *testing.T) {
	manifest := "[package]\nname = \"jackioh-x\"\nversion = \"0.1.0\"\n\n[dependencies]\n" +
		"serde = { workspace = true }\nserde_json.workspace = true\n\"indexmap\" = \"2\"\n" +
		"renamed = { package = \"real-name\", version = \"1\" }\n\n[[bin]]\nname = \"tool\"\npath = \"x.rs\"\n\n" +
		"[dev-dependencies]\nproptest = \"1\"\n\n[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n\n" +
		"[dependencies.tokio]\nversion = \"1\"\n\n[dependencies.alias]\npackage = \"aliased\"\n\n" +
		"[features]\ndefault = [\"serde\"]\n"
	assertEqual(t, mergegate.CargoDependencies(manifest),
		[]string{"serde", "serde_json", "indexmap", "real-name", "proptest", "libc", "tokio", "aliased"})
}

// ------------------------------------------------------------------ the pull request

// mergegateBuild builds issue 12 with `files` on top of a `main` that holds `onMain`, approved, and
// returns the pull request's number and what the bot said on the issue last.
func mergegateBuild(t *testing.T, onMain map[string]string, files map[string]any) (*flowHarness, int, string) {
	t.Helper()
	h := newFlowHarness(t)
	if len(onMain) > 0 {
		fakes.PushBranch(h.origin, h.root, "main", onMain)
		fakes.Git(h.clone, "fetch", "-q", "origin")
		fakes.Git(h.clone, "reset", "-q", "--hard", "origin/main")
		fakes.Git(h.deliverRepo, "fetch", "-q", "origin")
	}
	h.gh.AddIssue(12, fakes.Issue{Title: "Patch: a change", Labels: flowLabels(config.LabelBuild)})
	edits := map[string]any{"src/game.txt": "rules v2\n"}
	for k, v := range files {
		edits[k] = v
	}
	_, result := h.night(workFake(map[string]any{"build": workBuilder(edits), "review": workReviewer(workApprove)}))
	assertEqual(t, result.Get("status"), "approved")
	pull := h.gh.ListPulls("open", "bot/issue-12", 200)[0]
	return h, pull.Int("number"), lastOf(t, h.gh.BotComments(12))
}

// mergegateMerges is: the pull request's auto-merge is on, and it waits for no person.
func mergegateMerges(t *testing.T, h *flowHarness, number int, said string) {
	t.Helper()
	assertIn(t, h.gh.GetPull(number).Str("node_id"), h.gh.AutoMerge)
	assertFalse(t, h.gh.LabelNames(number)[config.LabelReady], said)
	assertIn(t, "Auto-merge is on", said)
}

// mergegateWaits is: the pull request waits for a person, labelled `ready for merge`, and the comment
// says why.
func mergegateWaits(t *testing.T, h *flowHarness, number int, said string, why ...string) {
	t.Helper()
	assertNotIn(t, h.gh.GetPull(number).Str("node_id"), h.gh.AutoMerge)
	assertTrue(t, h.gh.LabelNames(number)[config.LabelReady], said)
	assertIn(t, "labelled `ready for merge`: @jgoetzmann, it waits for you to merge it", said)
	for _, w := range why {
		assertIn(t, w, said)
	}
}

func TestMergegate_AMigrationWithNothingDataLosingOrAccessChangingMergesItself(t *testing.T) {
	h, number, said := mergegateBuild(t, nil, map[string]any{
		"crates/server/migrations/0080_news.sql": "-- the news\ncreate table public.news (id uuid primary key);\n" +
			"alter table public.news drop constraint if exists news_pkey;\n"})
	mergegateMerges(t, h, number, said)
	assertIn(t, "It merges by itself though it changes `crates/server/migrations/0080_news.sql`: "+
		"a migration with no data-losing or access-changing statement.", said)
}

func TestMergegate_AMigrationThatLosesDataOrChangesAccessWaitsForAPerson(t *testing.T) {
	h, number, said := mergegateBuild(t, nil, map[string]any{
		"crates/server/migrations/0080_drop.sql": "create table public.a (id int);\ndrop table public.decks;\n"})
	mergegateWaits(t, h, number, said, "the migration `crates/server/migrations/0080_drop.sql` loses data: "+
		"`drop table public.decks`")
	h, number, said = mergegateBuild(t, nil, map[string]any{
		"crates/server/migrations/0080_rls.sql": "create table public.a (id int);\n" +
			"alter table public.a enable row level security;\n"})
	mergegateWaits(t, h, number, said, "the migration `crates/server/migrations/0080_rls.sql` changes who may "+
		"read or write what: `alter table public.a enable row level security`")
}

func TestMergegate_AMigrationThatUpdatesEveryRowWaitsForAPerson(t *testing.T) {
	h, number, said := mergegateBuild(t, nil, map[string]any{
		"crates/server/migrations/0080_rank.sql": "alter table public.profiles add column rank int;\n" +
			"update public.profiles set rank = 0;\n"})
	mergegateWaits(t, h, number, said, "the migration `crates/server/migrations/0080_rank.sql` loses data: "+
		"`update public.profiles set rank = 0`")
	h, number, said = mergegateBuild(t, nil, map[string]any{
		"crates/server/migrations/0080_rank.sql": "alter table public.profiles add column rank int;\n" +
			"update public.profiles set rank = 0 where rank is null;\n"})
	mergegateMerges(t, h, number, said)
}

func TestMergegate_AnEditedOrDeletedMigrationWaitsForAPerson(t *testing.T) {
	old := map[string]string{"crates/server/migrations/0001_init.sql": "create table public.a (id int);\n"}
	h, number, said := mergegateBuild(t, old, map[string]any{
		"crates/server/migrations/0001_init.sql": "create table public.a (id bigint);\n"})
	mergegateWaits(t, h, number, said, "it edits `crates/server/migrations/0001_init.sql`, a migration `main` "+
		"already has, and an applied migration never changes")
	h, number, said = mergegateBuild(t, old, map[string]any{"crates/server/migrations/0001_init.sql": nil})
	mergegateWaits(t, h, number, said, "it deletes `crates/server/migrations/0001_init.sql`")
}

func TestMergegate_ALockThatGainsAPackageWaitsForAPerson(t *testing.T) {
	lock := "version = 4\n\n[[package]]\nname = \"memchr\"\nversion = \"2.7.4\"\n"
	manifest := "[package]\nname = \"x\"\n\n[dependencies]\nmemchr = \"2\"\n"
	onMain := map[string]string{"Cargo.lock": lock, "Cargo.toml": manifest}
	// A version change of a package the lock has merges itself.
	h, number, said := mergegateBuild(t, onMain, map[string]any{
		"Cargo.lock": strings.Replace(lock, "2.7.4", "2.7.5", 1)})
	mergegateMerges(t, h, number, said)
	assertIn(t, "`Cargo.lock`: no new package in the locks", said)
	// A new one, direct or transitive, goes to a person.
	h, number, said = mergegateBuild(t, onMain, map[string]any{
		"Cargo.lock": lock + "\n[[package]]\nname = \"regex\"\nversion = \"1.0.0\"\n",
		"Cargo.toml": manifest + "regex = \"1\"\n"})
	mergegateWaits(t, h, number, said, "`Cargo.lock` gains the package `regex`, which `main`'s lock does "+
		"not have: a new dependency")
	// A manifest naming a package its lock does not hold: CI builds with no `--locked`.
	h, number, said = mergegateBuild(t, onMain, map[string]any{"Cargo.toml": manifest + "regex = \"1\"\n"})
	mergegateWaits(t, h, number, said, "`Cargo.toml` depends on the package `regex`, which `Cargo.lock` "+
		"does not lock: a new dependency")
	// A manifest that changes no dependency merges itself.
	h, number, said = mergegateBuild(t, onMain, map[string]any{"Cargo.toml": manifest + "\n[profile.dev]\nopt-level = 1\n"})
	mergegateMerges(t, h, number, said)
	assertIn(t, "`Cargo.toml`: no new package in the locks", said)
}

func TestMergegate_APnpmLockThatGainsAPackageWaitsForAPerson(t *testing.T) {
	lock := "lockfileVersion: '9.0'\n\npackages:\n\n  vitest@3.2.4:\n    resolution: {integrity: y}\n"
	h, number, said := mergegateBuild(t, map[string]string{"pnpm-lock.yaml": lock, "package.json": "{}\n"},
		map[string]any{"pnpm-lock.yaml": lock + "\n  '@scope/new@1.0.0':\n    resolution: {integrity: z}\n",
			"package.json": "{\"devDependencies\": {\"@scope/new\": \"1\"}}\n"})
	mergegateWaits(t, h, number, said, "`pnpm-lock.yaml` gains the package `@scope/new`")
}

func TestMergegate_BuildScriptsTheDockerfileAndTheScriptsMergeThemselves(t *testing.T) {
	h, number, said := mergegateBuild(t, nil, map[string]any{"crates/cards/build.rs": "fn main() {}\n",
		"crates/server/Dockerfile": "FROM scratch\n", "scripts/build-wasm.sh": "echo wasm\n",
		"scripts/ci-scope.sh": "echo full=true\n", "package.json": "{}\n"})
	mergegateMerges(t, h, number, said)
	for _, script := range []string{"scripts/promote-production.sh", "scripts/vercel-ignore.sh"} {
		h, number, said = mergegateBuild(t, nil, map[string]any{script: "echo deploy\n"})
		mergegateWaits(t, h, number, said, "the change touches `"+script+"`, which a person reviews")
	}
}
