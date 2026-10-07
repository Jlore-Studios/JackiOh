# Spec gaps: part 27, chunk 1 of 8
TESTS-IN: 129 (after_resolution 12, combat_windows 22, condition_active 66, control_change_carry 2, control_change 20, costs_and_mana 7)

## SPEC GAPS
None: every `it` of the six TS files (and every iteration of the two `for`s in condition-active that
declare `it`s) has a Rust `#[test]` with the same assertions. No file reads source text, so none was
dropped under #133's rule. The names each test calls are listed under GAPS in `part-27-1.md`.
