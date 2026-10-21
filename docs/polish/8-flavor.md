# Card flavors

Every catalog entry has one `flavor` (spec §5): what kind of thing the card is, for animations
only. No rule, pool or AI reads it. It picks a card's projectile, cast, melee impact, summon
entrance, death exit and trap reveal (#1436). One flavor per entry; its Radiant face has the
same one.

| Flavor | What it covers | Ranged projectile | Motion |
|---|---|---|---|
| Physical | Trained fighters, soldiers, athletes, weapons, ordinary people, plain combat tricks. The fallback | an arrow | arc, pointing along its path |
| Brute | Size and raw force: giants, golems, walls, armour, heavy animals built to smash, quakes | a boulder | arc, spinning, a small shake on impact |
| Beast | Animals and animal people: Felinors, cats, snakes, bats, insects, shrimp; claws, bites, the hunt | a claw-slash crescent | swipe |
| Fire | Destructive magic: fire, lava, explosions, burns, spells that deal damage | a fireball | straight bolt with a trail |
| Dark | Evil magic: curses, the void, demons, doom, banishment, stealing life or cards | a dark bolt | straight bolt with a trail |
| Holy | Kind magic: healing, blessings, shields, radiance, protecting or buffing friends | sparkles | straight bolt, soft |
| Nerd | Study and knowledge: Books, professors, students, archives, draw, Discover, rules | a book | arc, spinning |
| Tech | Machines and computers: Jlockeed, AI, robots, gadgets, guns, factories | a laser | instant beam |
| Nature | Plants, fungi, growth, mulch, the wild | seeds or thorns | arc, a spread of three |
| Toxic | Plague, poison, slime, rot, stench, disease | a goo glob | arc with a drip trail |
| Snack | Food and drink: Pancakes, Fruit, cookies, Potions | a thrown snack | arc, spinning |
| Hustle | Money and schemes: gamblers, lobbyists, smugglers, grifters, lawyers, markets, deals, gacha | coins and bills | arc, scattering |
| Clout | Fame and the internet: influencers, fans, streamers, clips, memes, K-pop, media | hearts and emoji | arc, floating |
| Spirit | Ghosts and souls: phantoms, spirits, echoes, totems, the afterlife; eerie, not evil | a wisp | wavy bolt |
| Chaos | Randomness and tricksters: random effects, swaps, rotations, Call to Chaos | another flavor's projectile, picked from the instance id | that flavor's |

## Choosing one

The first rule that gives an answer decides.

1. **A tag.** Book → Nerd; Plague → Toxic; Jlockeed or AI → Tech; Pancake, Fruit or Potion →
   Snack; Felinor → Beast; Call to Chaos → Chaos. When an entry's tags give two different
   flavors, this rule gives none. The other tags (Human, CN, KY, Catalyst, Prime, Acclaimed,
   Quickdraw, Wincon, Token) give none.
2. **What it is, by its name.** A card whose name says what it is takes that: Gatling Pea is
   Nature though it shoots, The Rock is Brute though it is Human, Lava Golem is Fire, Paul Allen's
   Ghost is Spirit, Gary the Gambler is Hustle, K-Pop Fanatic is Clout.
3. **A Spell, Trap, Field Spell or Field Trap, by what it mostly does.** Damage → Fire; destroy,
   steal, discard, curse or Lock → Dark; heal, shield or buff your own → Holy; draw, Discover, copy
   or look at cards → Nerd; mana, cost, prices or markets → Hustle; random results, swaps or
   rotations → Chaos; summon → the flavor of what it summons; a field → the place it is (a garden
   is Nature).
4. **A token, by what it is**, never by the card that makes it.
5. **Physical.**

A pull request that assigns flavors lists every entry it touched in a table: id, name, type,
flavor, the rule that decided it, and for rule 2 or 3 a few words why.

## Looks

Every flavor's look lives in one table, `FLAVOR_LOOKS` in `apps/web/src/fx/flavors.ts`: its two or three colours, its particle shape and its projectile, with `chaosFlavor` there picking the look a Chaos card takes from its instance id (never Chaos itself), and `flavorOf` in `apps/web/src/cards/flavor.ts` the one lookup of which flavor a definition draws (R1437000). Each projectile is built from four motions (arc, bolt, beam, swipe, drawn by `fx/shots.ts`), and a Ranged attacker stays in place and fires its flavor's projectile at its target, the hit landing as the entry ends. `FX_FLAVOR_LIVE` in `fx/constants.ts` keeps all of it dark, every cue today's, until #1444 turns it on for the last part of #1436.
