# Brief: translate one area of the web client (PR #736, issue #711)

Temporary working file of the lead; deleted before the pull request is delivered.

## Context

PR #736 adds a reader language to `apps/web` (English and 简体中文, `apps/web/src/i18n/`). A reviewer
blocked it: most screens and the engine's and server's English still read in English for a Chinese
reader. Your job is one area of the client: wrap what is still English and write the Chinese for it.

Read first: the header and API of `apps/web/src/i18n/i18n.ts` (`t`, `useLocale`, `formatNumber`,
`formatDateTime`, `formatDay`, `formatSeconds`), and look at how already-converted files do it
(`apps/web/src/main.tsx`, `apps/web/src/routes/play.tsx`, `apps/web/src/game/Prompt.tsx`).

## What to do in each file you own

1. Wrap every user-visible English string in `t()`: JSX text; `aria-label`, `title`, `placeholder`
   and `alt` values; labels and hints passed as props; and the sentences built in `.ts` files that
   reach the screen (errors, statuses, labels, tooltips, announcements).
2. Write the Chinese for every key your files pass to `t()` that `apps/web/src/i18n/zh-CN.json`
   lacks, into your fragment `i18n-parts/<area>.json` (a flat JSON object, exact English key to its
   Chinese). Write the fragment as you go, after each file, so the work survives an interruption.
   Do not edit `zh-CN.json` or `i18n.ts`; the lead merges the fragments.

Leave English: test ids, `data-*` attributes, class names, React keys, URLs, console/log messages,
strings code compares or uses as ids, keyboard key names, usernames, the voice lines (recorded
audio), the patch notes' own text (from `patches.json`), the privacy policy's and terms' body (only
their "The English text governs." line is translated), the brand "JackiOh", and `routes/dev/`.

## Rules (the reviewer checks them)

- **English must read exactly as before.** `t(english)` returns the English when there is no entry,
  so `t("Your turn")` is safe. With parameters, `t("Pay {n} mana", { n })` must produce the old
  string byte for byte. Keep whitespace inside JSX identical (text content tests compare it).
  Every existing test must pass untouched: never edit a test or a snapshot.
- **Never call `t()` at module scope** (a top-level constant, a map built at import): it freezes the
  language. Keep the English constant and call `t(CONSTANT[key])` where it is rendered, and put each
  of the constant's values in your fragment (the scanner only sees literal `t("…")` calls, so add
  those keys by hand).
- **Plurals:** keep the code's existing English branch (`n === 1 ? t("{n} card", { n }) : t("{n}
  cards", { n })`) and give both keys their Chinese (the same words, usually).
- **A sentence with an element inside it** (a link, `<code>`, `<strong>`): one entry per sentence.
  Either split the translated sentence around a token, as `NotFound` in `main.tsx` does
  (`t("There is nothing at {path}. …").split("{path}")`), or, when the parts really are separate
  phrases, wrap each. The English text content must not change.
- **Re-rendering on a switch:** `App` (main.tsx) calls `useLocale()`, so a plain component redraws.
  A component under `memo`, or text computed in `useMemo`/`useState`/`useRef`/an effect, must call
  `useLocale()` itself and have the locale in its deps.
- **Display sites of engine or server English** (`{prompt.title}`, `{error}`, `{issue.message}`, an
  API error's message): wrap them as `t(value)`. Area H writes the dictionary for those sentences;
  name each such site in your report.
- Do not change test ids, `data-*` attributes, class names or the element structure tests query.
- Import as the repo does: `import { t } from "../i18n/i18n.ts";` (relative, with `.ts`).
- Match the surrounding code; reformat nothing you do not need to touch. No `eslint-disable`,
  `@ts-ignore`, `@ts-expect-error`, no new dependency.
- Number and date formatting: replace a bare `toLocaleString()` or a raw ISO date shown to a reader
  with the helpers in `i18n.ts`, but only when English reads the same after (check it).

## Translation style

Simplified Chinese, in Hearthstone's zh-CN words where it has them; card words come from
`crates/cards/chinese-terms.json` (types, keywords, labels, rarities, glossary: read it). Fixed
vocabulary, so every area reads alike:

| English | 中文 |
|---|---|
| Unit | 单位 |
| Spell / Trap / Field Spell | 法术 / 陷阱 / 场地法术 |
| hero | 英雄 |
| mana | 法力值 (a cost: 法力值消耗; "(N) Cost" → "(N)费") |
| Attack / Health | 攻击力 / 生命值 |
| deck (saved, in the builder) | 卡组 |
| deck (in a game, the draw pile) | 牌库 |
| hand | 手牌 |
| graveyard | 墓地 |
| lane | 纵列 |
| backrow | 后场 |
| Tribute | 献祭 |
| Stack | 堆叠 |
| Radiant | 光辉 |
| Token | 衍生物 |
| turn / end turn | 回合 / 结束回合 |
| draw (a card) / draw (a result) | 抽牌 / 平局 |
| Mulligan | 换牌 |
| concede | 认输 |
| opponent | 对手 |
| match / game | 对局 |
| Best of 1 / Conquest / All Random | 单局 / 征服 / 全随机 |
| trio | 三套卡组 |
| practice / tutorial / lesson | 练习模式 / 教程 / 课程 |
| rank / rating / leaderboard | 段位 / 积分 / 排行榜 |
| Card Almanac | 卡牌图鉴 |
| patch notes | 更新日志 |
| sign in / sign out / account | 登录 / 退出登录 / 账户 |
| email / password / username | 邮箱 / 密码 / 用户名 |
| invite code | 邀请码 |
| emote | 表情 |
| Settings | 设置 |

Keep every `{token}` of the key in the value, spelled the same. Chinese punctuation in Chinese text
(，。：；！？（）…——). Keep arrows (← →), symbols and digits.

## Checking your work

- `node i18n-parts/scan.mjs "$PWD" --missing <path-filter>` (run from the repo root, the filter
  relative to `apps/web/src`, e.g. `routes/login`) lists the literal `t("…")` keys in matching files
  that neither `zh-CN.json` nor any fragment has. It must print nothing for your files.
  `--unwrapped <path-filter>` lists JSX text and `aria-label`/`title`/`placeholder`/`alt` literals
  not yet wrapped; whatever it still shows for your files must be something the rules above leave
  English (say which in your report). It does not see sentences built in `.ts` files: read them.
- `pnpm --dir apps/web typecheck` and `pnpm exec eslint <your files>`: other agents edit other files
  at the same time, so judge only errors in your files (report any you see elsewhere).
- `pnpm --dir apps/web exec vitest run <the test files beside your files>`: all must pass. Do not
  run the whole suite. The WASM module is already built.

Never run git commands that change state (no commit, stash, checkout, reset, add). Edit only your
files and your fragment.

## Your report

The files you changed; what you left English and why; each display site of engine or server text
you wrapped; the commands you ran and their results.
