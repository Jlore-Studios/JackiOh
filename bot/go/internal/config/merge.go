package config

// The merge gates on a condition (#2986, section 1; the owner's decision of 2026-11-04, amended the
// same day; `deliver.Deliverer.mergeGates`, `internal/mergegate`). A path on `review_paths` always
// waits for a person; these merge themselves unless what they hold says otherwise:
//
//   - `migration_paths`: a migration merges itself unless it loses data or changes who may read or
//     write what, or edits or deletes a migration `main` already has.
//   - `lock_files`: a lock merges itself unless it gains a package name `main`'s lock lacks; a
//     Cargo manifest a change touches must name only packages its lock holds.
//
// A config that leaves them out checks neither: a path neither list nor `review_paths` names merges
// itself.
func init() {
	optional.Set("migration_paths", []any{})
	optional.Set("lock_files", []any{})
}
