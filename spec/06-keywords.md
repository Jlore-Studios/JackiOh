## 6. Keyword glossary

Every keyword below maps to one engine primitive; a card script only ever composes these primitives.

Players read two of this spec's words differently (v0.1.1, [[R373]]): the rules' library is shown to players as the Deck, and Sacrifice as Tribute. The rules, the engine's identifiers and this spec keep "library" and "Sacrifice".

Two verbs were renamed for players and in this spec alike (patch v0.3.4, [[R1320]]): Degrade is **Nerf** and Upgrade is **Buff** ([[§6.3]]). The engine's identifiers keep the old names (`degrade`, `upgrade`, `tune_once`, `TuneDirection`, the events `degraded` and `upgraded`, the wire names), so a ruling or a section written before the rename that says Degrade or Upgrade means Nerf or Buff.

One word changed meaning (issue #1839): **Dormant** is Stinkyscape's keyword ([[§6.1]], [[R1839000]]), and a card under a Stack pile's top, which this spec called dormant, is now a card **beneath a Stack** ([[§3.2]]); a face-down card's text that was dormant is inert. The engine's identifiers for a card beneath a Stack keep the old word (`dormant_units_of`, `alt_play::is_dormant`, `preview::lasting_text_dormant`, the AI's `dormant_share` and `dormant_card`), and the keyword's own say slumber and awaken (`slumber.rs`, `CardInstance.slumber`, `awakened`), so neither is read for the other.

A card's text prints every [[§6.2]] trigger and [[§6.3]] verb, and Radiant, in bold wherever it uses the word as that term, in every case and inflection ("exile it", "exiled", "Locked", "Tributes"); a word spelled like a term that names something else, such as the exile pile ("your exile"), stays plain, a [[§6.1]] keyword bolds as the keyword a card has, and the rows the glossary keeps rules-only (Summon, Draw, Discard, Destroy and the like) stay plain ([[R910000]]). How a face spells its terms and writes the rest of its words is [[§5.4]] ([[R3941000]]).
