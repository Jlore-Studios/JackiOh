# `luau/`

The Luau side of the engine's C boundary (#4218, part of #2898). `crates/wasm/c`
is the C boundary for a host that is not JavaScript; this folder is the Luau
that runs against it once the port translates it. It follows
[`crates/wasm/c/README.md`](../crates/wasm/c/README.md) exactly, and the JSON of
every binding is [`crates/wasm/README.md`](../crates/wasm/README.md)'s, the same
as the browser module's.

## Layout

| File | Holds |
| --- | --- |
| `src/Host.luau` | `Host.new` / `host:call` / `Host.splitState`: the boundary |
| `tests/mock.luau` | A mock module in Luau over a real `buffer`, the spec's oracle |
| `tests/host.spec.luau` | The spec: a tiny assert helper, no framework |
| `tools/reference-host.mjs` | Node reference host driving `jackioh_wasm_c.wasm` through the C ABI (`fixture` and `traces` modes; run by `scripts/build-wasm.sh`) |
| `rokit.toml` | The pinned tools |
| `stylua.toml` | The formatter's config |
| `selene.toml` | The linter's config |

## Tools

| Tool | Version | Use |
| --- | --- | --- |
| `lune-org/lune` | `0.11.1` | Runs the spec |
| `JohnnyMorganz/StyLua` | `2.6.0` | Formats `luau/` |
| `Kampfkarren/selene` | `0.32.0` | Lints `luau/` |

Install them with [Rokit](https://rokit.rocks) on `PATH`:

```
cd luau && rokit install
```

## Tests

From the repo root:

```
lune run luau/tests/host.spec.luau
stylua --check luau
selene luau
```

All three are clean on a passing tree.

## Notes

- `Host.new(instance)` takes `{ exports, memory: () -> buffer }` and calls
  `init()` once. `instance` is an adapter over whatever the translator
  produces; that adapter arrives with the translation, in a later issue.
- JSON stays opaque text. `Host` decodes nothing, and `Host.splitState` cuts
  the `state` member out of an answer as its exact text, never re-encoded.
