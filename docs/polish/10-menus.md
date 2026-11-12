# Polish 10: the menus' shell and the conventions they share

Issue #3728, part of the tracker #3726. Rulings: **R3728000**, **R3728001** and **R3728002**
([Rulings](#rulings)). The tracker's other UI issues build to the conventions below and cite them by
number.

## Goal

The menus get one frame, the app shell, around every menu screen: one navigation in one place, one
status row, and one friends drawer. The screens inside it keep their own content; the other issues
of #3726 redesign each one.

Desktop and iPad come first, both ways up, with or without a keyboard and a trackpad. A phone must
still look right, but comes second. Layouts key on the window's width, never on the device: Split
View and Stage Manager put an iPad window at any width.

## Research

### What the best do

What each does, and what we take from it:

- **Hearthstone.** The hub is one screen of a few big doors, and the collection is a grid of cards
  that fills the screen, with its filters at the edge. *We take:* few doors, and a grid that uses the
  width.
- **Legends of Runeterra.** A handful of top-level destinations in one navigation that every menu
  screen shares, and a friends list that opens over the screen you're on, grouped by status. *We
  take:* five destinations, and friends that never move you.
- **Marvel Snap.** A tab bar of a few destinations along the bottom, and on the home tab one big Play
  button that nothing else competes with. *We take:* five destinations, and one primary action per
  screen.
- **Clash Royale.** The tabs run along the bottom of the phone, a swipe moves between them, and
  Battle is the one big button. *We take:* the bottom tab bar on a phone, in reach of a thumb.
- **Discord.** Friends sit in lists by status, and a red count appears only for what waits on you: a
  message, a mention, a friend request. *We take:* a badge only for what needs you, always a count.
- **Steam.** The friends list groups people as in a game, online and offline, shows what a friend in
  a game is playing, and folds the offline ones away. *We take:* the grouping, with Offline folded
  under its count.
- **Lichess.** The challenge dialog asks for the variant, the time and whether it's rated in one
  place, and opens next time on the last choices. Correspondence games waiting on your move are
  easy to find from the home page. *We take:* setup in one dialog with remembered defaults, and the
  games that wait on you in plain sight.
- **Apple's iPadOS guidelines.** A sidebar on a regular-width window and a tab bar on a compact one,
  chosen by size class, not by device. Targets are at least 44×44 points, content stays inside the
  safe areas, and the pointer's hover is an extra, never the only way. *We take:* the rail and the tab
  bar by width, `--hit-min`, `env(safe-area-inset-*)`, and hover as a bonus.

### The counter-example

Chess.com: a cluttered home, daily games buried, settings scattered, and a game's setup spread over
several screens. We do the opposite on each count. Lichess, which does the same jobs, is much
cleaner.

## Conventions

1. **One navigation, always in the same place.** The top-level destinations are Home, Play,
   Collection, Shop and Friends: five, as Snap and LoR keep it. Play carries the badge for the daily
   games waiting on you (not built: daily games don't exist yet).

   | Width | Who | The navigation |
   |---|---|---|
   | Wide, 1025 px and wider | a desktop, an iPad on its side | a left rail of icons with labels |
   | Medium, 601–1024 px | an upright iPad | the same rail, icons only |
   | Compact, 600 px and under | a phone | a tab bar along the bottom |

   The two lines are `apps/web/src/breakpoints.ts`'s `phone` (600) and `tablet` (1024). Each
   breakpoint is the widest window of the smaller layout, so the issue's "at least 1024 / 600–1023 /
   under 600" became 1025 and wider / 601–1024 / 600 and under. No size in [The sizes](#the-sizes)
   is 1024 or 600 px wide, so the move changes none of them.
2. **A badge only for what needs you.** A badge is a count, and it counts only what waits on you:
   your move, a challenge, a friend request, an invite. News and shop offers never get one.
3. **One primary action per screen.** It wears `button-primary`, the primary role in
   `apps/web/src/buttons.css`. A screen has at most one.
4. **Status that never hides.** A search in the queue, a live game to go back to and a lobby you're
   in each show in one banner slot under the top bar, on every menu screen, with Cancel or Return.
   Built now: the live game (Rejoin) and the queue search (Leave the queue), in the top bar's status
   row (`.screen-nav__status`) and on Home under its corner. The lobby is not: no lobby status exists
   off /play yet. The "Match found!" notice (`QueueFound`) stays where it is.
5. **Friends never take you off the screen.** Friends opens a drawer from the right edge:
   - on a wide window, beside the screen: over 1024 px the screen's column narrows to make room, and
     nothing dims;
   - on a medium window, over the screen;
   - on a phone, as a full-width sheet.

   It shows friends grouped by status, as Steam and LoR do. Today it holds the existing friends panel
   as it is; the friends issue redesigns what is inside. The full page at /together (Play together)
   is for managing them. The drawer closes on Escape, on Back and on a tap outside it.
6. **Setup in one dialog, with remembered defaults.** Lichess's challenge dialog asks for the
   variant, the time and rated in one place, and remembers them. A challenge, a new daily game and a
   lobby's settings all follow it: one dialog that opens on your last choices. Never a wizard.
7. **Lists before grids before chrome.** Content fills the width: a menu with a list or a grid in it
   has no frame narrower than the window. Filters go in a sidebar at wide widths and in a sheet below
   them. Widening the account page's `panel--auth` frame (at most 960 px) is left to the screen
   issues.
8. **Touch first, hover a bonus.** Every target is at least 44×44 px (`--hit-min`), as Apple's
   guidelines ask. Nothing works only on hover: an iPad with a trackpad gets the hover, and one
   without still gets everything. Drag works with a finger too: pointer events, never HTML5
   `draggable`.
9. **Safe areas everywhere.** The rail, the tab bar, the top bar, the status row, toasts and drawers
   all keep inside `env(safe-area-inset-*)`. The challenge toasts sit on the `--z-toast` layer token,
   inside the insets, clear of the rail and the tab bar. A toast fixed at the foot of a screen's column
   (the deck builder's status, the undo toast) reads the shell's `--shell-start` and `--shell-bottom` to
   center on the column and sit above the tab bar.
10. **The game's look stays.** The shell is structure, not a new skin. It keeps the tavern
    (`apps/web/src/auth/tavern.css`), the palette, the type, surface and motion tokens, and the voice
    in `apps/web/WRITING.md`.

## Wireframes

### Wide

The issue's wide window, with the drawer open beside the screen:

```
┌──────┬───────────────────────────────────────────────┬───────────────┐
│ Home │  Play                        ◆ 1,240  🔔 3  ⚙ │ Friends     ✕ │
│▶Play③│  In queue: Best of 1 · 0:42 · ~1 min   Cancel │ Requests    1 │
│ Coll │ ┌───────────────────────────────────────────┐ │ In a game     │
│ Shop │ │                                           │ │  Ana    Watch │
│ Frnds│ │          (the screen's content)           │ │ Online        │
│      │ │                                           │ │  Bo Challenge │
│  me  │ └───────────────────────────────────────────┘ │ Offline (12)  │
└──────┴───────────────────────────────────────────────┴───────────────┘
```

The bell, the daily badge (③) and the account item ("me") are not built ([Out of scope](#out-of-scope)).

### Medium

An upright iPad: the rail, about 80 px, holds icons only, and the drawer lies over the screen.

```
┌─────┬──────────────────────────────────┬───────────────┐
│ [H] │  Play                            │ Friends     ✕ │
│▶[P]③│  In queue: Best of 1 · 0:42      │ Requests    1 │
│ [C] │ ┌────────────────────────────────│ In a game     │
│ [S] │ │                                │  Ana    Watch │
│ [F] │ │  (the screen, under the drawer)│ Online        │
│     │ │                                │  Bo Challenge │
│     │ └────────────────────────────────│ Offline (12)  │
└─────┴──────────────────────────────────┴───────────────┘
```

### Compact

The issue's phone:

```
┌──────────────────────────┐
│ ‹  Play       ◆1,240 🔔3 │
│ In queue · 0:42  Cancel  │
│                          │
│     (content, scrolls)   │
│                          │
├──────────────────────────┤
│ Home Play③ Coll Shop Frnd│
└──────────────────────────┘
```

The bell and the daily badge (③) are not built.

### The screens

One sketch per screen the other issues redesign, at a wide width. Each is a sketch for the screen's
issue to settle.

**Home.** The tavern's three doors are Home's content, with the rail around it; the status sits under
the corner.

```
┌──────┬────────────────────────────────────────────────────────┐
│▶Home │  Home                                       ◆ 1,240  ⚙ │
│ Play │                               You're in a game  Rejoin │
│ Coll │                                                        │
│ Shop │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│ Frnds│  │     Play     │  │  Collection  │  │     Shop     │  │
│      │  │              │  │              │  │              │  │
│      │  └──────────────┘  └──────────────┘  └──────────────┘  │
│      │  (the landing's other parts, as today)                 │
└──────┴────────────────────────────────────────────────────────┘
```

**Play.** The hub: a mode picker and one primary action, Find a match.

```
┌──────┬────────────────────────────────────────────────────────┐
│ Home │  Play                                       ◆ 1,240  ⚙ │
│▶Play │                                                        │
│ Coll │  ◉ Best of 1      ○ Conquest      ○ All random         │
│ Shop │                                                        │
│ Frnds│  Deck: Aggro ▾                  Usually under 1 min    │
│      │                                                        │
│      │                   [ Find a match ]                     │
│      │                                                        │
│      │  Practice · Dungeon run · Play together · Replays      │
└──────┴────────────────────────────────────────────────────────┘
```

**Collection.** A filter sidebar at wide widths (a sheet below them) and a grid that fills the width. Four
tabs, Cards, Decks, Cosmetics and Progress, each keeping its filters while you switch
([R3734000](../../spec/rulings/R3734000.md)); the search is always on show.

```
┌──────┬────────────────────────────────────────────────────────┐
│ Home │  Collection                                 ◆ 1,240  ⚙ │
│ Play │  ▸Cards   Decks   Cosmetics   Progress                 │
│▶Coll │ ┌──────────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ │
│ Shop │ │ Search   │ │    │ │    │ │    │ │    │ │    │ │    │ │
│ Frnds│ │ Cost     │ └────┘ └────┘ └────┘ └────┘ └────┘ └────┘ │
│      │ │ Type     │ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ │
│      │ │ Set      │ │    │ │    │ │    │ │    │ │    │ │    │ │
│      │ └──────────┘ └────┘ └────┘ └────┘ └────┘ └────┘ └────┘ │
└──────┴────────────────────────────────────────────────────────┘
```

**Shop.** The offers in a grid that fills the width; the odds and the pity stay on their own page.

```
┌──────┬────────────────────────────────────────────────────────┐
│ Home │  Shop                                       ◆ 1,240  ⚙ │
│ Play │  Card packs                                            │
│ Coll │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐   │
│▶Shop │  │          │ │          │ │          │ │          │   │
│ Frnds│  └──────────┘ └──────────┘ └──────────┘ └──────────┘   │
│      │  Cosmetic packs                                        │
│      │  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│      │  └──────────┘ └──────────┘ └──────────┘                │
│      │  Odds & pity                                           │
└──────┴────────────────────────────────────────────────────────┘
```

**The friends drawer.** Grouped: Requests, In a game, Online, Offline (n).

```
┌────────────────────────────┐
│ Friends                  ✕ │
├────────────────────────────┤
│ Requests                 1 │
│   Cy        Accept Decline │
│ In a game                  │
│   Ana                Watch │
│ Online                     │
│   Bo             Challenge │
│ ▸ Offline (12)             │
├────────────────────────────┤
│ Open Play together         │
└────────────────────────────┘
```

## The sizes

Every menu layout spec runs one list, `MENU_SIZES` in `e2e/support/config.ts`:

| Window | Sizes | Layout |
|---|---|---|
| Desktop | 1440×900, 1280×720 | wide |
| iPad on its side | 1180×820, 1366×1024 | wide |
| Upright iPad | 820×1180, 744×1133 (iPad mini) | medium |
| Phone | 390×844 | compact |

Spec 46 (every menu fits one screen), 47 (tap areas), 51 (the shell: every destination one tap away,
the drawer, screenshots) and 38's axe block over the shell run it.

## What the shell is

- `apps/web/src/shell/MenuShell.tsx` wraps every menu route in `main.tsx`.
  `apps/web/src/shell/context.ts` hands the screens inside it the status row, and `testids.ts` names
  its chrome.
- The boards stay outside it: `isBoardPath` in `apps/web/src/net/navigate.ts` (practice, matches,
  watching, co-op, the dev boards). So do the series screen `/series/:id`, which is part of a game's
  flow, and the dev screens.
- A board that appears on a menu path takes the navigation away.
- The workshop's three-column split starts at 601 px, the rail, the pool and the deck all drawn on an
  upright iPad (issue #3734, [R3734001](../../spec/rulings/R3734001.md)): its rail is 208 px, and 148 px
  from 601 to 1024 px. At 600 px and under the list and the editor take turns. A card is dragged from
  the pool to the deck by pointer events, so a finger works.
- On nested screens the top bar keeps Back top left and the gear top right (R3218000), with the title
  treatment of #3223 (R3223000).

## Rulings

- [R3728000](../../spec/rulings/R3728000.md): the shell. Which paths it wraps, the three widths, the
  board stepping aside, the screen height a menu fills, the workshop's split and the insets. It
  amends [R3218000](../../spec/rulings/R3218000.md) (Friends leaves the top bar for the rail) and
  [R1581000](../../spec/rulings/R1581000.md).
- [R3728001](../../spec/rulings/R3728001.md): the friends drawer. It amends
  [R3221000](../../spec/rulings/R3221000.md).
- [R3728002](../../spec/rulings/R3728002.md): the status row and the toasts. It amends
  [R3218000](../../spec/rulings/R3218000.md) and [R1566000](../../spec/rulings/R1566000.md).

## Out of scope

Left to the other issues of #3726:

- The bell.
- The daily badge on Play: daily games don't exist yet.
- The lobby's status in the status row.
- An account item ("me") at the rail's foot.
- Widening `panel--auth`.
- What the friends drawer holds.
- Each screen's own redesign. The sketches above are where those issues start.
