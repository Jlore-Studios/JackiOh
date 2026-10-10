## 1. Overview

JackiOh is a 1v1 collectible card game: a Hearthstone-style mana curve, combat math and keyword vocabulary, played on Yu-Gi-Oh-style lanes with a hidden backrow of traps. This document is the complete specification: the rules, all 370 cards of the four sets (100 Core, 90 Classic, 78 Classic+, 102 Meditative) and their 80 tokens described by what they do to game state, and the architecture and engine they run on. Patch v0.2.0 (issue #40, [[R380]] on) added Classic and Classic+, and the Meditative set (issue #496) shipped with issue #553 ([[R1420]]).

Design pillars:

- Short, sharp games: 20-card decks with no duplicates, 4 max mana, 30 hero health, a 60-turn cap that is a backstop, fatigue being the usual end of a long game ([[§2.5]]).
- Two combat axes: Attack/Defense position (Defense = Taunt + Armor) layered over Hearthstone-style free-target attacking.
- Radiant: every card has an upgraded form, and upgrading cards mid-game is a core resource loop (the Glowy Jelly Bean family, Radiant Saintess, Gifted Program, Eugenics).
- Lanes matter: 5 shared lanes give adjacency (Cleave, Hit Job, Collateral Damage), rotation (Silly Silas) and per-lane traps (Zoomerbin Oomen).
- Chaos is a feature: coin flips, random keywords, Call to Chaos, Pocket Chaos, Transmogulate. All of it must run on a seeded RNG so any match replays exactly.

How to read this spec: sections 2 to 7 are the rules; 8 is the card catalog (mechanics, not flavor text), with each Core card's rarity assigned by mechanical complexity and each Classic and Classic+ card's rarity the designer's; 9 and 10 are the architecture and the engine design; 11 collects every ruling this spec makes where the source was silent or ambiguous, with the ones still open marked decide.

Source material: the original JackiOh design notes (a mechanics sheet, the Core card list and a CCG architecture document), and for Classic and Classic+ the designer's card list of 2026-09-30 (`JackiOh_Classic_Cards.md`, whose reading card by card is the design brief `docs/classic-sets.md`), referred to below as "the source". Wherever this spec goes beyond them it is marked **Ruling:** in place, and the same item appears in section 11 so it can be confirmed or overturned in one place. Card references read "#N" for Core, "C #N" for Classic and "C+ #N" for Classic+, and "T-AI-N" for the AI generated cards ([[§7]]).
