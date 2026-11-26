# Infinite Assault sweep, 2026-11-26 (#4595)

R186/R390's two-pass sweep over the Infinite Assault, the held set (`sweep` previews a held set's cards as it
will ship them, R1420): all 240 cards at easy and hard, each on its base and Radiant face, then a second pass
over every card the first put at risk. **This is not a report of record.** `sweep --write` refuses the set's
bans (its `shadow_ban_share` is `None`, IA11; R1678000), so `crates/ai/src/shadow_ban.rs` keeps its tables.
It is #4595's measurement of the AI with the set: what it misplayed, what was fixed (R4595001–R4595004), and
the fixed cards measured again.

## How it ran

- **Tree**: `main` at `c0b49712` (#4885). Cards 001–171 of pass 1 ran earlier on #4872's merge (`c739a07d`).
  The AI differs between the two only by #4885 (R4803000, which redacts an Attack below 0), so the two halves
  of pass 1 are one measurement.
- **Machine**: a Claude Code cloud container, 4 cores (Intel(R) Xeon(R) Processor @ 2.10GHz). Pass 1 ran on
  4 lanes (one `sweep --json <id>` per card, `xargs -P 4`), about 75 minutes for cards 172–240. Pass 2 ran on
  3 lanes over the 139 at-risk cards, from about 05:20 to 22:45 UTC on 2026-11-25, through two container
  restarts. After each restart the unfinished cards were run again from scratch; no partial card's lines
  were kept.
- **Errors**: one, Fata Morgana (#35), easy. It is fixed by R4595001.
- **Timeouts run alone** (Step 7, R977000's re-timer against the perf gate's yardstick, with nothing else on
  the cores): Cost Injection (#123), Encrypt (#149) and Defense in Depth (#161) read `"timeouts":0` on all four
  lines when run alone, so their timeouts came from the lanes' contention. False Positive (#29) and Ego Death
  (#48, easy) were still slow alone; they are fixed by R4595003 and R4595004.
- **Raw lines**: the per-card JSON lines are not committed (each pass-2 card is about 190 KB). They are on
  the branch `claude/issue-4595-assault-ai-2` (`sweep-partial-4595/`); the id lists are in
  `training/history/assault-sweep-4595/`.

## AI shadow-ban sweep

Run 2026-11-25 (UTC): pass 1 over 240 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 139 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), each card also on its Radiant face (`sweep:<tier>:<id>:radiant:<n>`, `sweep2:<tier>:<id>:radiant:<n>`), budget AI_GATE_BUDGET {"nodes":1200,"lethalNodes":300,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}; 8 banned, 131 watched, 0 unswept, 75 suspect line(s).

### Pass 1 flags (a `neverPlayed`, `selfHarm` or `selfKill` here only puts a card at risk)

| id | name | tier | face | flags | drawn/games | affordable turns | playable turns | plays | errors | timeouts | raw timeouts | mean eval delta |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infiniteassault-005 | Generated Filler | hard | base | neverPlayed | 2/8 | 4 | 4 | 0 | 0 | 0 | 3 | n/a |
| infiniteassault-018 | Prompt Engineer | hard | radiant | neverPlayed | 3/8 | 8 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-019 | Infinite Scroll | hard | base | neverPlayed | 4/8 | 20 | 20 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-019 | Infinite Scroll | hard | radiant | neverPlayed | 3/8 | 12 | 12 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-020 | Synthetic Data | hard | base | neverPlayed | 3/8 | 13 | 13 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-021 | Endless Production Line | hard | radiant | neverPlayed | 4/8 | 6 | 6 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-029 | False Positive | easy | base | timeout | 3/8 | 9 | 9 | 1 | 0 | 2 | 10 | 1.0 |
| infiniteassault-037 | Bluff Called | hard | radiant | neverPlayed | 3/8 | 12 | 11 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-040 | Hall of Mirrors | hard | base | neverPlayed | 2/8 | 3 | 3 | 0 | 0 | 0 | 16 | n/a |
| infiniteassault-043 | Cognitive Dissonance | hard | base | neverPlayed | 1/8 | 4 | 4 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-043 | Cognitive Dissonance | hard | radiant | neverPlayed | 2/8 | 10 | 10 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-045 | Dream Within a Dream | easy | base | neverPlayed | 3/8 | 4 | 4 | 0 | 0 | 0 | 18 | n/a |
| infiniteassault-045 | Dream Within a Dream | hard | radiant | neverPlayed | 3/8 | 5 | 5 | 0 | 0 | 0 | 5 | n/a |
| infiniteassault-047 | Unreal Estate | hard | base | neverPlayed | 3/8 | 12 | 12 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-048 | Ego Death | hard | base | neverPlayed | 3/8 | 8 | 8 | 0 | 0 | 0 | 2 | n/a |
| infiniteassault-065 | Overdrive | easy | base | neverPlayed | 2/8 | 5 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-065 | Overdrive | easy | radiant | neverPlayed | 3/8 | 13 | 13 | 0 | 0 | 0 | 9 | n/a |
| infiniteassault-067 | Burning Ambition | hard | base | neverPlayed | 1/8 | 5 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-067 | Burning Ambition | hard | radiant | neverPlayed | 4/8 | 14 | 14 | 0 | 0 | 0 | 3 | n/a |
| infiniteassault-072 | Overclock to Infinity | easy | base | neverPlayed | 5/8 | 16 | 16 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-072 | Overclock to Infinity | easy | radiant | neverPlayed | 5/8 | 8 | 8 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-072 | Overclock to Infinity | hard | base | neverPlayed | 3/8 | 11 | 11 | 0 | 0 | 0 | 8 | n/a |
| infiniteassault-072 | Overclock to Infinity | hard | radiant | neverPlayed | 3/8 | 11 | 11 | 0 | 0 | 0 | 5 | n/a |
| infiniteassault-079 | Thread Pool | hard | base | neverPlayed | 4/8 | 15 | 10 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-084 | Distributed Mind | hard | base | neverPlayed | 1/8 | 4 | 3 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-085 | Merge Conflict | easy | base | neverPlayed | 4/8 | 14 | 10 | 0 | 0 | 0 | 17 | n/a |
| infiniteassault-085 | Merge Conflict | hard | radiant | neverPlayed | 2/8 | 6 | 4 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-090 | Mass Production | hard | base | neverPlayed | 2/8 | 7 | 7 | 0 | 0 | 0 | 2 | n/a |
| infiniteassault-107 | Recursive Call | hard | base | neverPlayed | 3/8 | 14 | 14 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-108 | Second Draft | hard | base | neverPlayed | 1/8 | 3 | 3 | 0 | 0 | 0 | 1 | n/a |
| infiniteassault-109 | Revenant Process | hard | base | neverPlayed | 2/8 | 6 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-110 | Infinite Loop | easy | radiant | neverPlayed | 3/8 | 19 | 19 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-110 | Infinite Loop | hard | base | neverPlayed | 3/8 | 11 | 11 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-113 | Rerun | hard | base | neverPlayed | 3/8 | 12 | 12 | 0 | 0 | 0 | 1 | n/a |
| infiniteassault-116 | Groundhog Day | hard | radiant | neverPlayed | 3/8 | 8 | 8 | 0 | 0 | 0 | 2 | n/a |
| infiniteassault-117 | Core Dump | easy | radiant | neverPlayed | 2/8 | 10 | 10 | 0 | 0 | 0 | 22 | n/a |
| infiniteassault-120 | Eternal Return | easy | radiant | neverPlayed | 2/8 | 6 | 6 | 0 | 0 | 0 | 23 | n/a |
| infiniteassault-120 | Eternal Return | hard | base | neverPlayed | 5/8 | 12 | 12 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-123 | Cost Injection | easy | base | neverPlayed | 3/8 | 7 | 7 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-123 | Cost Injection | hard | base | timeout, neverPlayed | 2/8 | 7 | 7 | 0 | 0 | 2 | 4 | n/a |
| infiniteassault-123 | Cost Injection | hard | radiant | neverPlayed | 3/8 | 7 | 7 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-124 | Firewall Breach | hard | radiant | neverPlayed | 4/8 | 9 | 9 | 0 | 0 | 0 | 2 | n/a |
| infiniteassault-127 | Bad Prompt | easy | base | neverPlayed | 6/8 | 19 | 19 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-127 | Bad Prompt | easy | radiant | neverPlayed | 4/8 | 17 | 17 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-127 | Bad Prompt | hard | radiant | neverPlayed | 2/8 | 7 | 7 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-129 | Signal Jammer | easy | base | neverPlayed | 5/8 | 9 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-130 | Prompt Injection | easy | radiant | neverPlayed | 7/8 | 30 | 30 | 0 | 0 | 0 | 7 | n/a |
| infiniteassault-130 | Prompt Injection | hard | base | neverPlayed | 3/8 | 6 | 6 | 0 | 0 | 0 | 1 | n/a |
| infiniteassault-132 | Ransomware | hard | base | neverPlayed | 5/8 | 17 | 17 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-132 | Ransomware | hard | radiant | neverPlayed | 2/8 | 10 | 10 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-137 | Patch Tuesday Exploit | easy | base | neverPlayed | 4/8 | 21 | 21 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-137 | Patch Tuesday Exploit | hard | base | neverPlayed | 3/8 | 11 | 11 | 0 | 0 | 0 | 1 | n/a |
| infiniteassault-137 | Patch Tuesday Exploit | hard | radiant | neverPlayed | 5/8 | 25 | 25 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-141 | Logic Bomb | hard | base | neverPlayed | 6/8 | 21 | 20 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-144 | Total Compromise | easy | base | neverPlayed | 1/8 | 3 | 3 | 0 | 0 | 0 | 1 | n/a |
| infiniteassault-148 | Quarantine | hard | radiant | neverPlayed | 1/8 | 5 | 5 | 0 | 0 | 0 | 1 | n/a |
| infiniteassault-149 | Encrypt | hard | base | timeout | 3/8 | 6 | 6 | 1 | 0 | 4 | 12 | 3.7 |
| infiniteassault-154 | Honeypot Protocol | easy | base | neverPlayed | 2/8 | 12 | 12 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-158 | Access Denied | hard | radiant | neverPlayed | 3/8 | 11 | 10 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-160 | Zero Trust | hard | base | neverPlayed | 3/8 | 12 | 8 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-164 | Blackhole Route | hard | radiant | neverPlayed | 6/8 | 21 | 20 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-167 | Fortress Protocol | hard | base | neverPlayed | 2/8 | 4 | 4 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-168 | Impenetrable | hard | base | neverPlayed | 4/8 | 11 | 11 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-169 | Spider Bot | easy | base | neverPlayed | 2/8 | 6 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-172 | Training Data | hard | base | neverPlayed | 4/8 | 12 | 12 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-172 | Training Data | hard | radiant | neverPlayed | 2/8 | 6 | 6 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-188 | Overfitting | hard | radiant | neverPlayed | 1/8 | 3 | 3 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-192 | Become the Data | easy | base | neverPlayed | 4/8 | 12 | 12 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-192 | Become the Data | hard | base | neverPlayed | 1/8 | 5 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-192 | Become the Data | hard | radiant | neverPlayed | 1/8 | 3 | 3 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-228 | Firewall Rules | hard | radiant | neverPlayed | 5/8 | 14 | 13 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-240 | Unpatched | easy | base | neverPlayed | 4/8 | 14 | 14 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-240 | Unpatched | hard | base | neverPlayed | 2/8 | 7 | 7 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-240 | Unpatched | hard | radiant | neverPlayed | 4/8 | 9 | 8 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-213 | The Long Game | hard | base | neverPlayed | 3/8 | 7 | 6 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-213 | The Long Game | hard | radiant | neverPlayed | 7/8 | 18 | 14 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-186 | Identity Theft | hard | base | neverPlayed | 4/8 | 15 | 15 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-226 | Escalated Privileges | hard | radiant | neverPlayed | 4/8 | 12 | 12 | 0 | 0 | 0 | 2 | n/a |
| infiniteassault-179 | Style Transfer | hard | base | neverPlayed | 2/8 | 6 | 6 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-183 | Shadow Library | hard | base | neverPlayed | 3/8 | 15 | 15 | 0 | 0 | 0 | 1 | n/a |
| infiniteassault-187 | Fine-Tuner | easy | base | neverPlayed | 2/8 | 10 | 10 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-187 | Fine-Tuner | hard | radiant | neverPlayed | 3/8 | 10 | 9 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-207 | Recursive Self-Improvement | hard | base | neverPlayed | 2/8 | 5 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-211 | Grey Goo | hard | base | neverPlayed | 8/8 | 29 | 22 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-211 | Grey Goo | hard | radiant | neverPlayed | 6/8 | 24 | 18 | 0 | 0 | 0 | 0 | n/a |

### At risk (395): pass 1 at half strength, SHADOW_BAN and SHADOW_WATCH

classic-006 Cloaked Toe Cracker, classic-015 Nose Hunter, classic-018 Glitch in the System, classic-026 Rapid Draw, classic-037 Last Hurrah, classic-038 Jackiestan Auctioneer, classic-042 Transmutable Toxins, classic-043 Plague Nuke, classic-044 Back from the GY, classic-050 Voidwalker, classic-055 Book of Wildfire, classic-062 Living Bomb, classic-065 Ace in the Hole, classic-066 EU Striker, classic-070 Book of Plague, classic-074 Corpse Plantation, classic-078 Mutate Spell, classic-080 BOOM! Big Max, classic-089 Paul Allen's Ghost, classicplus-003 Second Amendment Snake, classicplus-009 Silence, classicplus-014 Forever&, classicplus-017 Conjure Rush Token++, classicplus-022 Blood Moon, classicplus-029 Portal to the Past, classicplus-030 Felinor Fuser, classicplus-031 Fusion Lab, classicplus-033 Ivory Tower, classicplus-034 Memory Leak, classicplus-036 Conjure Bones, classicplus-041 KY's Constant, classicplus-042 KY's Test, classicplus-043 AI Slop, classicplus-044 Simplicity Audit, classicplus-045 Complexity Audit, classicplus-048 Jlockeed's Lobbyist, classicplus-065 Two Grapes, classicplus-066 Vine of Grapes, classicplus-073 Call to Chaos (Classic+ Edition), classicplus-078 Claude's Datacenter, core-001 Big D-fender, core-005 Stockpile, core-006 Mana Well, core-007 Jewelosco Scarab, core-016 Hit Job, core-017 Flood, core-022 Carnivorous Cube, core-026 Glowy Jelly Bean, core-034 Collateral Damage, core-040 Echoes of the Forgotten, core-042 Selective Breeding, core-048 5pek Controller, core-057 Conjure KY, core-058 Rush Token Farm, core-059 Unbiased Immigration, core-065 Masochism Mask, core-069 Call to Arms, core-072 Reminisce, core-079 Twinspell, core-083 Transmogulate, core-086 "Miss" Mrow, core-087 Pocket Chaos, core-090 Super-Virus Injection, core-092 Felinor Fiender, core-093 Combo-Index, core-094 Genn's Greed, core-099 Craft a Card, infiniteassault-005 Generated Filler, infiniteassault-010 Assault Carrier, infiniteassault-011 Quickgen Turret, infiniteassault-012 Recompile, infiniteassault-013 Training Run, infiniteassault-015 Drone Foundry, infiniteassault-017 Cluster Drone, infiniteassault-018 Prompt Engineer, infiniteassault-019 Infinite Scroll, infiniteassault-020 Synthetic Data, infiniteassault-021 Endless Production Line, infiniteassault-022 Assault Mainframe, infiniteassault-023 The Generator, infiniteassault-024 Mass Deployment, infiniteassault-027 Decoy Projector, infiniteassault-029 False Positive, infiniteassault-034 Mass Hallucination, infiniteassault-035 Fata Morgana, infiniteassault-037 Bluff Called, infiniteassault-039 Shell Game, infiniteassault-040 Hall of Mirrors, infiniteassault-041 Phantom Army, infiniteassault-042 Paranoia Engine, infiniteassault-043 Cognitive Dissonance, infiniteassault-045 Dream Within a Dream, infiniteassault-047 Unreal Estate, infiniteassault-048 Ego Death, infiniteassault-050 Burst Clock, infiniteassault-052 Thermal Paste, infiniteassault-053 Overvolt Grunt, infiniteassault-056 Afterburner, infiniteassault-058 Overclocked Core, infiniteassault-061 Meltdown Charge, infiniteassault-062 Burnout, infiniteassault-065 Overdrive, infiniteassault-067 Burning Ambition, infiniteassault-068 Short Circuit, infiniteassault-071 Thermal Throttler, infiniteassault-072 Overclock to Infinity, infiniteassault-078 Batch Job, infiniteassault-079 Thread Pool, infiniteassault-084 Distributed Mind, infiniteassault-085 Merge Conflict, infiniteassault-086 Replica Hatchery, infiniteassault-088 Scale Out, infiniteassault-090 Mass Production, infiniteassault-091 Flood the Zone, infiniteassault-092 Hive Overseer, infiniteassault-093 Gestalt, infiniteassault-095 The Botnet, infiniteassault-098 Echo Chamber, infiniteassault-099 Recall, infiniteassault-100 Iterator, infiniteassault-103 Undo, infiniteassault-105 Callback, infiniteassault-106 Checkpoint Daemon, infiniteassault-107 Recursive Call, infiniteassault-108 Second Draft, infiniteassault-109 Revenant Process, infiniteassault-110 Infinite Loop, infiniteassault-111 Redundant Copy, infiniteassault-112 Evolutionary Loop, infiniteassault-113 Rerun, infiniteassault-114 Ouroboros Engine, infiniteassault-116 Groundhog Day, infiniteassault-117 Core Dump, infiniteassault-118 The Recursionist, infiniteassault-119 Snapshot Lich, infiniteassault-120 Eternal Return, infiniteassault-123 Cost Injection, infiniteassault-124 Firewall Breach, infiniteassault-125 Zone Lock, infiniteassault-127 Bad Prompt, infiniteassault-129 Signal Jammer, infiniteassault-130 Prompt Injection, infiniteassault-132 Ransomware, infiniteassault-134 Rootkit Runner, infiniteassault-135 Mana Leech, infiniteassault-136 DDoS, infiniteassault-137 Patch Tuesday Exploit, infiniteassault-138 Social Engineer, infiniteassault-140 Kill Switch, infiniteassault-141 Logic Bomb, infiniteassault-143 Zero-Click, infiniteassault-144 Total Compromise, infiniteassault-146 Shield Drone, infiniteassault-148 Quarantine, infiniteassault-149 Encrypt, infiniteassault-150 Wall of Static, infiniteassault-154 Honeypot Protocol, infiniteassault-157 Containment Field, infiniteassault-158 Access Denied, infiniteassault-160 Zero Trust, infiniteassault-161 Defense in Depth, infiniteassault-164 Blackhole Route, infiniteassault-165 Iron Curtain, infiniteassault-166 Aegis Prime, infiniteassault-167 Fortress Protocol, infiniteassault-168 Impenetrable, infiniteassault-169 Spider Bot, infiniteassault-171 Mimic Unit, infiniteassault-172 Training Data, infiniteassault-174 Scrape the Graveyard, infiniteassault-175 Fingerprinter, infiniteassault-176 Imitation Engine, infiniteassault-177 Predictive Text, infiniteassault-178 Dataset Thief, infiniteassault-179 Style Transfer, infiniteassault-182 Model Theft, infiniteassault-183 Shadow Library, infiniteassault-184 Pattern Recognition, infiniteassault-185 Deep Crawler, infiniteassault-186 Identity Theft, infiniteassault-187 Fine-Tuner, infiniteassault-188 Overfitting, infiniteassault-189 Clone Vat, infiniteassault-191 Ground Truth, infiniteassault-192 Become the Data, infiniteassault-194 Compute Shard, infiniteassault-195 Power Law, infiniteassault-196 Emergent Behavior, infiniteassault-199 Patient Machine, infiniteassault-202 Exponent, infiniteassault-203 Compute Cluster, infiniteassault-207 Recursive Self-Improvement, infiniteassault-211 Grey Goo, infiniteassault-213 The Long Game, infiniteassault-214 Emergent Mind, infiniteassault-218 Thousand Faces, infiniteassault-220 Last Lap, infiniteassault-223 Loopback, Who Returns, infiniteassault-224 Rollback Point, infiniteassault-226 Escalated Privileges, infiniteassault-228 Firewall Rules, infiniteassault-234 Degenerate Output, infiniteassault-236 Infinite Assault, infiniteassault-237 Thermal Runaway, infiniteassault-238 Critical Temperature, infiniteassault-240 Unpatched, meditative-011 Double Header, meditative-022 Mind Games, meditative-026 Alternate Fate, meditative-053 Prestige, meditative-055 Dragon Fruit, meditative-060 Eschews, meditative-069 The Maestro, meditative-075 Ever Growing Tree, meditative-098 Showdown, meditative-099 Paranoia, meditativeplus-002 NA Tech, meditativeplus-003 Wondrous Wand, meditativeplus-004 Jlockeed's Supply Drop, meditativeplus-005 Spirit Lash, meditativeplus-013 Living Health Potion, meditativeplus-014 Potion Pouch, meditativeplus-020 KY's Competition, meditativeplus-022 Mountaintop Garden, meditativeplus-023 Enormous Elderberry, stinkyscape-002 Absolute Zero, stinkyscape-004 Olivian Expanse, stinkyscape-007 The Greatest Heist, stinkyscape-008 Conjure Rosemary Triscuit, stinkyscape-009 Ticketed, stinkyscape-010 Otherworldly Endurance, stinkyscape-011 Otherworldly Magic, stinkyscape-017 Imprisoned Ship Smith, stinkyscape-019 Imprisoned Cosmality, stinkyscape-021 Moonberry, stinkyscape-022 Gatinsir, stinkyscape-024 Warp Disruptor II, stinkyscape-032 Drone Core VI, stinkyscape-036 Drone Core X, stinkyscape-039 The Narcor Project, stinkyscape-040 Poison Capsule, stinkyscape-046 Circadian Halfcycler, stinkyscape-047 Circadian Fullcycler, stinkyscape-049 Spacemine, stinkyscape-052 Cannon v3, stinkyscape-055 Repeater v4, stinkyscape-058 Beam v5, stinkyscape-059 Repeater v5, stinkyscape-061 Flak v5, stinkyscape-063 Miner v2, stinkyscape-064 Miner v3, stinkyscape-065 Miner v4, stinkyscape-066 Miner v5, stinkyscape-071 A'Zoth, stinkyscape-074 Paragon-X, stinkyscape-076 Honey Badger, stinkyscape-080 Galaxy, stinkyscape-082 Phalanx, stinkyscape-083 Halo, stinkyscape-084 Power Cell, stinkyscape-085 Prototype-X1, stinkyscape-086 Prototype-X2, stinkyscape-087 Siva 1, stinkyscape-088 Siva 2, stinkyscape-089 Spice, stinkyscape-093 Patcher Nanobots, stinkyscape-095 Faction: Vanguard Coalition, stinkyscape-096 Ocean Tsunami, stinkyscape-104 Reflective Plating, stinkyscape-109 xxFreshJack, stinkyscape-114 Arctic Citadel, stinkyscape-115 Scornboi, stinkyscape-117 Core System, stinkyscape-118 Secure System, stinkyscape-119 Unsecure System, stinkyscape-120 Wild System, stinkyscape-121 System Schematic, stinkyscape-124 Jlockeed's Shipbuilding Grant, stinkyscape-125 Jlockeed's Spacerace Propaganda, stinkyscape-126 Jlockeed's Permanent Procurement Lobbying, stinkyscape-127 Jlockeed's Siege Tank, stinkyscape-128 Jlockeed's Yamato Cannon, stinkyscape-130 Jlockeed's Starship Retrofitting Program, stinkyscape-131 Spacerace Stimulus, stinkyscape-132 Defensive Crystal, stinkyscape-133 Ultra Capacitor, stinkyscape-134 Felinor Containment Chamber, stinkyscape-141 Fruit Launcher, stinkyscape-144 Prepare For Liftoff, stinkyscape-145 Fortify Flesh, stinkyscape-150 Call to Chaos (Stinky Edition), stinkyscape-151 Call to Chaos (Scape Edition), stinkyscape-152 Sanctum, stinkyscape-155 Comet Crash, stinkyscape-156 Catalyst Comet, stinkyscape-157 The Gauntlet, stinkyscape-158 Stratos Swarm, stinkyscape-159 Faction: ETHICS, stinkyscape-161 Faction: Pegasus, stinkyscape-162 Faction: Star-Jumper Core, stinkyscape-163 Swear Loyalities, stinkyscape-164 The Syndicate, stinkyscape-165 The Trade Union, stinkyscape-166 Scanning Probe Mk1, stinkyscape-167 Scanning Probe Mk2, stinkyscape-171 Large Asteroid, stinkyscape-172 Huge Asteroid, stinkyscape-173 Abandoned Spice Tower, stinkyscape-174 Aerelret, stinkyscape-175 Frost Module, stinkyscapeplus-001 Starfruit, stinkyscapeplus-004 Profe Yates, stinkyscapeplus-005 Dingleberry Droat, stinkyscapeplus-008 Drone Assembler v3, stinkyscapeplus-009 Drone Assembler v4, stinkyscapeplus-013 Shield Repairer, stinkyscapeplus-015 Hull Repairer mk2, stinkyscapeplus-023 GIGA-ify, stinkyscapeplus-024 Forced Propagation, stinkyscapeplus-026 Jlockeed’s Forever War, stinkyscapeplus-030 Yeah I’m a War Hero, stinkyscapeplus-032 Z-Patch, stinkyscapeplus-033 True Conservative, stinkyscapeplus-038 Arc, stinkyscapeplus-040 Mender, stinkyscapeplus-041 Mend Projector, stinkyscapeplus-043 Segment, stinkyscapeplus-046 Phase Fabric, stinkyscapeplus-049 Metaglass, stinkyscapeplus-050 Graphite, stinkyscapeplus-052 Airblast Drill, stinkyscapeplus-053 Laser Drill, stinkyscapeplus-054 Pneumatic Drill, stinkyscapeplus-056 Impact Reactor, stinkyscapeplus-057 Phase Weaver, stinkyscapeplus-060 Radi, stinkyscapeplus-063 Overdrive Dome, stinkyscapeplus-068 Shadiris Visions, stinkyscapeplus-069 Exotic Jellybean, stinkyscapeplus-072 Foreshadow, stinkyscapeplus-073 Ripple, stinkyscapeplus-077 Black Hole, stinkyscapeplus-079 A Glimpse into the Endless Sea, stinkyscapeplus-085 Body Snatch, stinkyscapeplus-087 Frontier Flora, stinkyscapeplus-098 Doomed Ship, stinkyscapeplus-103 Transcend, stinkyscapeplus-106 Spectre, stinkyscapeplus-109 Lancer, stinkyscapeplus-111 Mace, stinkyscapeplus-113 Scepter, stinkyscapeplus-119 Eclipse, stinkyscapeplus-123 Oct, stinkyscapeplus-125 Omura, stinkyscapeplus-126 Vela, stinkyscapeplus-129 Multiplicative Reconstructor, stinkyscapeplus-130 Exponential Reconstructor, stinkyscapeplus-131 Tetrative Reconstructor, stinkyscapeplus-133 Great Paperclip Theory III, stinkyscapeplus-136 Crawler Assembly Line, stinkyscapeplus-138 Delta Definition, stinkyscapeplus-140 Automa Reintegration Plan, stinkyscapeplus-142 Call to Chaos (Mindustrial-Swarm Edition), stinkyscapeplus-143 Fire Beam v3, stinkyscapeplus-144 Fire Beam v4, stinkyscapeplus-145 Fire Beam v5, stinkyscapeplus-146 Scourge Mortar v3, stinkyscapeplus-147 Scourge Mortar v4, stinkyscapeplus-148 Scourge Mortar v5, stinkyscapeplus-150 Scorching Rounds, stinkyscapeplus-151 Volcanic Rounds, stinkyscapeplus-153 Noxious Rounds, stinkyscapeplus-155 The Far Vagabond, stinkyscapeplus-156 Horizon Breaker, stinkyscapeplus-160 Icarus, stinkyscapeplus-165 Shadiris, stinkyscapeplus-166 Mindustrial Revolution, traininggrounds-007 Chosen by the Moon, traininggrounds-012 Fortify, traininggrounds-013 Explosive Runes, traininggrounds-016 Celestial Caller, traininggrounds-017 Flaming Felinor, traininggrounds-019 Extra Rations, traininggrounds-020 Recruitment Posters, traininggrounds-031 Level Up!, traininggrounds-035 Corpse Recycler, traininggrounds-036 Training Dummy, traininggrounds-037 Seance, traininggrounds-038 Tutorial Master, traininggrounds-046 Legacy: Hit Job, traininggrounds-049 Legacy: Reoccurring Dream, traininggrounds-050 Legacy: Glowy Jelly Bean, traininggrounds-053 Legacy: Prejudiced Postdoc, traininggrounds-055 Legacy: Reminisce, traininggrounds-060 Legacy: Rush Token Farm

### Pass 2: every at-risk card over every game it was dealt in, forced or filler

| id | name | tier | face | flags | drawn/games | affordable turns | playable turns | plays | errors | timeouts | raw timeouts | mean eval delta |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| classic-006 | Cloaked Toe Cracker | easy | base | cleared | 184/372 | 285 | 281 | 159 | 0 | 0 | 0 | 8.9 |
| classic-006 | Cloaked Toe Cracker | hard | base | cleared | 229/510 | 416 | 401 | 170 | 0 | 0 | 0 | 9.2 |
| classic-015 | Nose Hunter | easy | base | cleared | 231/455 | 449 | 374 | 185 | 0 | 0 | 0 | 4.9 |
| classic-015 | Nose Hunter | hard | base | cleared | 202/499 | 401 | 335 | 133 | 0 | 0 | 0 | 5.4 |
| classic-018 | Glitch in the System | easy | base | cleared | 94/190 | 196 | 197 | 71 | 0 | 0 | 0 | 23.0 |
| classic-018 | Glitch in the System | hard | base | cleared | 145/299 | 401 | 401 | 82 | 0 | 0 | 0 | 17.8 |
| classic-026 | Rapid Draw | easy | base | cleared | 123/225 | 164 | 164 | 110 | 0 | 0 | 0 | 0.8 |
| classic-026 | Rapid Draw | hard | base | cleared | 74/209 | 140 | 140 | 45 | 0 | 0 | 0 | 0.4 |
| classic-037 | Last Hurrah | easy | base | cleared | 114/225 | 96 | 96 | 102 | 0 | 0 | 0 | 9.7 |
| classic-037 | Last Hurrah | hard | base | cleared | 100/211 | 104 | 103 | 70 | 0 | 0 | 0 | 7.0 |
| classic-038 | Jackiestan Auctioneer | easy | base | cleared | 82/171 | 177 | 168 | 67 | 0 | 0 | 0 | 5.9 |
| classic-038 | Jackiestan Auctioneer | hard | base | cleared | 106/244 | 244 | 243 | 72 | 0 | 0 | 0 | 5.6 |
| classic-042 | Transmutable Toxins | easy | base | cleared | 103/209 | 184 | 180 | 83 | 0 | 0 | 0 | 3.1 |
| classic-042 | Transmutable Toxins | hard | base | cleared | 142/311 | 311 | 296 | 101 | 0 | 0 | 0 | 2.4 |
| classic-043 | Plague Nuke | easy | base | cleared | 104/192 | 250 | 252 | 40 | 0 | 0 | 0 | 24.9 |
| classic-043 | Plague Nuke | hard | base | cleared | 156/338 | 408 | 408 | 37 | 0 | 0 | 0 | 34.9 |
| classic-044 | Back from the GY | easy | base | cleared | 72/134 | 129 | 129 | 29 | 0 | 0 | 0 | -0.6 |
| classic-044 | Back from the GY | hard | base | cleared | 127/271 | 359 | 359 | 44 | 0 | 0 | 0 | -1.4 |
| classic-050 | Voidwalker | easy | base | cleared | 164/330 | 244 | 216 | 144 | 0 | 0 | 0 | 11.9 |
| classic-050 | Voidwalker | hard | base | cleared | 211/475 | 398 | 302 | 136 | 0 | 0 | 0 | 11.6 |
| classic-055 | Book of Wildfire | easy | base | cleared | 148/290 | 129 | 129 | 92 | 0 | 0 | 0 | 8.5 |
| classic-055 | Book of Wildfire | hard | base | cleared | 148/337 | 133 | 133 | 93 | 0 | 0 | 0 | 5.5 |
| classic-062 | Living Bomb | easy | base | cleared | 141/237 | 305 | 299 | 97 | 0 | 0 | 0 | 2.2 |
| classic-062 | Living Bomb | hard | base | cleared | 150/367 | 398 | 388 | 75 | 0 | 0 | 0 | 1.7 |
| classic-065 | Ace in the Hole | easy | base | cleared | 83/159 | 235 | 232 | 55 | 0 | 0 | 0 | 1.8 |
| classic-065 | Ace in the Hole | hard | base | cleared | 105/242 | 288 | 286 | 60 | 0 | 0 | 0 | 2.1 |
| classic-066 | EU Striker | easy | base | cleared | 199/344 | 315 | 238 | 113 | 0 | 0 | 0 | 12.3 |
| classic-066 | EU Striker | hard | base | cleared | 211/493 | 395 | 296 | 66 | 0 | 0 | 0 | 11.7 |
| classic-070 | Book of Plague | easy | base | cleared | 175/325 | 292 | 292 | 157 | 0 | 0 | 0 | 2.8 |
| classic-070 | Book of Plague | hard | base | cleared | 153/355 | 248 | 248 | 116 | 0 | 0 | 0 | 2.9 |
| classic-074 | Corpse Plantation | easy | base | cleared | 113/215 | 312 | 310 | 81 | 0 | 0 | 0 | 1.5 |
| classic-074 | Corpse Plantation | hard | base | cleared | 127/285 | 433 | 401 | 66 | 0 | 0 | 0 | 1.4 |
| classic-078 | Mutate Spell | easy | base | cleared | 157/300 | 371 | 353 | 104 | 0 | 0 | 0 | 0.9 |
| classic-078 | Mutate Spell | hard | base | cleared | 140/302 | 345 | 337 | 91 | 0 | 0 | 0 | 0.8 |
| classic-080 | BOOM! Big Max | easy | base | cleared | 171/354 | 297 | 144 | 94 | 0 | 0 | 0 | 15.3 |
| classic-080 | BOOM! Big Max | hard | base | cleared | 289/616 | 626 | 377 | 181 | 0 | 0 | 0 | 14.6 |
| classic-089 | Paul Allen's Ghost | easy | base | cleared | 193/352 | 250 | 204 | 173 | 0 | 0 | 0 | 15.2 |
| classic-089 | Paul Allen's Ghost | hard | base | cleared | 231/473 | 321 | 257 | 188 | 0 | 0 | 0 | 15.1 |
| classicplus-003 | Second Amendment Snake | easy | base | cleared | 217/420 | 348 | 305 | 172 | 0 | 0 | 0 | 6.6 |
| classicplus-003 | Second Amendment Snake | hard | base | cleared | 243/522 | 500 | 406 | 149 | 0 | 0 | 0 | 5.8 |
| classicplus-009 | Silence | easy | base | cleared | 127/212 | 287 | 287 | 105 | 0 | 0 | 0 | 4.7 |
| classicplus-009 | Silence | hard | base | cleared | 89/211 | 172 | 172 | 62 | 0 | 0 | 0 | 1.9 |
| classicplus-014 | Forever& | easy | base | cleared | 106/204 | 529 | 529 | 55 | 0 | 0 | 0 | -1.1 |
| classicplus-014 | Forever& | hard | base | cleared | 107/251 | 408 | 408 | 19 | 0 | 0 | 0 | -0.7 |
| classicplus-017 | Conjure Rush Token++ | easy | base | cleared | 80/136 | 114 | 114 | 52 | 0 | 0 | 0 | 12.4 |
| classicplus-017 | Conjure Rush Token++ | hard | base | cleared | 131/280 | 257 | 257 | 72 | 0 | 0 | 0 | 12.6 |
| classicplus-022 | Blood Moon | easy | base | cleared | 88/167 | 280 | 277 | 55 | 0 | 0 | 0 | 1.7 |
| classicplus-022 | Blood Moon | hard | base | cleared | 117/260 | 406 | 393 | 43 | 0 | 0 | 0 | 1.5 |
| classicplus-029 | Portal to the Past | easy | base | cleared | 103/190 | 365 | 365 | 40 | 0 | 0 | 0 | -1.8 |
| classicplus-029 | Portal to the Past | hard | base | cleared | 154/314 | 468 | 467 | 40 | 0 | 0 | 0 | -2.0 |
| classicplus-030 | Felinor Fuser | easy | base | cleared | 166/329 | 200 | 146 | 98 | 0 | 0 | 0 | 6.1 |
| classicplus-030 | Felinor Fuser | hard | base | cleared | 269/570 | 443 | 335 | 168 | 0 | 0 | 0 | 6.3 |
| classicplus-031 | Fusion Lab | easy | base | cleared | 69/137 | 245 | 243 | 39 | 0 | 0 | 0 | 1.2 |
| classicplus-031 | Fusion Lab | hard | base | cleared | 105/238 | 355 | 348 | 45 | 0 | 0 | 0 | 1.4 |
| classicplus-033 | Ivory Tower | easy | base | cleared | 111/204 | 134 | 127 | 41 | 0 | 0 | 0 | 1.8 |
| classicplus-033 | Ivory Tower | hard | base | cleared | 104/246 | 123 | 120 | 34 | 0 | 0 | 0 | 2.6 |
| classicplus-034 | Memory Leak | easy | base | cleared | 109/230 | 263 | 255 | 75 | 0 | 0 | 0 | 2.6 |
| classicplus-034 | Memory Leak | hard | base | cleared | 114/242 | 242 | 234 | 74 | 0 | 0 | 0 | 0.9 |
| classicplus-036 | Conjure Bones | easy | base | cleared | 95/171 | 371 | 371 | 46 | 0 | 0 | 0 | -3.1 |
| classicplus-036 | Conjure Bones | hard | base | cleared | 131/261 | 506 | 506 | 38 | 0 | 0 | 0 | -2.0 |
| classicplus-041 | KY's Constant | easy | base | cleared | 148/289 | 628 | 628 | 85 | 0 | 0 | 0 | -1.4 |
| classicplus-041 | KY's Constant | hard | base | cleared | 136/297 | 418 | 417 | 61 | 0 | 0 | 0 | -0.6 |
| classicplus-042 | KY's Test | easy | base | cleared | 171/323 | 279 | 279 | 137 | 0 | 0 | 0 | -1.3 |
| classicplus-042 | KY's Test | hard | base | cleared | 131/307 | 263 | 263 | 87 | 0 | 0 | 0 | -1.2 |
| classicplus-043 | AI Slop | easy | base | cleared | 95/175 | 346 | 346 | 31 | 0 | 0 | 0 | -0.0 |
| classicplus-043 | AI Slop | hard | base | cleared | 131/281 | 434 | 434 | 31 | 0 | 0 | 0 | 0.1 |
| classicplus-044 | Simplicity Audit | easy | base | cleared | 89/184 | 237 | 237 | 52 | 0 | 0 | 0 | 28.8 |
| classicplus-044 | Simplicity Audit | hard | base | cleared | 144/305 | 473 | 473 | 53 | 0 | 0 | 0 | 23.5 |
| classicplus-045 | Complexity Audit | easy | base | cleared | 70/152 | 249 | 249 | 31 | 0 | 0 | 0 | 24.6 |
| classicplus-045 | Complexity Audit | hard | base | cleared | 105/240 | 365 | 364 | 27 | 0 | 0 | 0 | 12.1 |
| classicplus-048 | Jlockeed's Lobbyist | easy | base | cleared | 243/497 | 532 | 422 | 198 | 0 | 0 | 0 | 1.9 |
| classicplus-048 | Jlockeed's Lobbyist | hard | base | cleared | 225/515 | 522 | 406 | 130 | 0 | 0 | 0 | 2.0 |
| classicplus-065 | Two Grapes | easy | base | cleared | 146/271 | 269 | 269 | 123 | 0 | 0 | 0 | 1.2 |
| classicplus-065 | Two Grapes | hard | base | cleared | 144/328 | 283 | 283 | 103 | 0 | 0 | 0 | 1.6 |
| classicplus-066 | Vine of Grapes | easy | base | cleared | 134/241 | 379 | 379 | 77 | 0 | 0 | 0 | 4.6 |
| classicplus-066 | Vine of Grapes | hard | base | cleared | 201/391 | 649 | 649 | 65 | 0 | 0 | 0 | 2.9 |
| classicplus-073 | Call to Chaos (Classic+ Edition) | easy | base | cleared | 116/216 | 180 | 181 | 60 | 0 | 0 | 0 | 8.8 |
| classicplus-073 | Call to Chaos (Classic+ Edition) | hard | base | cleared | 169/358 | 384 | 384 | 71 | 0 | 0 | 0 | 8.3 |
| classicplus-078 | Claude's Datacenter | easy | base | cleared | 92/180 | 278 | 269 | 47 | 0 | 0 | 0 | 2.0 |
| classicplus-078 | Claude's Datacenter | hard | base | cleared | 139/293 | 464 | 443 | 55 | 0 | 0 | 0 | 2.1 |
| core-001 | Big D-fender | easy | base | cleared | 196/365 | 395 | 325 | 151 | 0 | 0 | 0 | 6.9 |
| core-001 | Big D-fender | hard | base | cleared | 209/482 | 497 | 384 | 101 | 0 | 0 | 0 | 5.9 |
| core-005 | Stockpile | easy | base | cleared | 109/205 | 180 | 180 | 87 | 0 | 0 | 0 | 3.0 |
| core-005 | Stockpile | hard | base | cleared | 101/248 | 176 | 176 | 76 | 0 | 0 | 0 | 3.0 |
| core-006 | Mana Well | easy | base | cleared | 89/184 | 332 | 319 | 52 | 0 | 0 | 0 | 1.0 |
| core-006 | Mana Well | hard | base | cleared | 117/242 | 406 | 390 | 43 | 0 | 0 | 0 | 1.3 |
| core-007 | Jewelosco Scarab | easy | base | cleared | 203/407 | 441 | 368 | 161 | 0 | 0 | 0 | 2.0 |
| core-007 | Jewelosco Scarab | hard | base | cleared | 213/476 | 472 | 349 | 121 | 0 | 0 | 0 | 1.4 |
| core-016 | Hit Job | easy | base | cleared | 92/190 | 193 | 193 | 75 | 0 | 0 | 0 | 27.6 |
| core-016 | Hit Job | hard | base | cleared | 144/318 | 325 | 326 | 89 | 0 | 0 | 0 | 14.9 |
| core-017 | Flood | easy | base | cleared | 66/130 | 141 | 141 | 19 | 0 | 0 | 0 | 51.1 |
| core-017 | Flood | hard | base | cleared | 124/279 | 353 | 353 | 22 | 0 | 0 | 0 | 35.1 |
| core-022 | Carnivorous Cube | easy | base | cleared | 180/345 | 612 | 527 | 101 | 0 | 0 | 0 | 6.2 |
| core-022 | Carnivorous Cube | hard | base | cleared | 213/465 | 741 | 593 | 61 | 0 | 0 | 0 | 6.1 |
| core-026 | Glowy Jelly Bean | easy | base | cleared | 144/274 | 226 | 226 | 29 | 0 | 0 | 0 | 5.9 |
| core-026 | Glowy Jelly Bean | hard | base | cleared | 161/355 | 234 | 234 | 30 | 0 | 0 | 0 | 8.1 |
| core-034 | Collateral Damage | easy | base | cleared | 75/148 | 87 | 87 | 42 | 0 | 0 | 0 | 23.0 |
| core-034 | Collateral Damage | hard | base | cleared | 138/295 | 290 | 290 | 85 | 0 | 0 | 0 | 19.9 |
| core-040 | Echoes of the Forgotten | easy | base | cleared | 83/162 | 321 | 319 | 46 | 0 | 0 | 0 | 1.5 |
| core-040 | Echoes of the Forgotten | hard | base | cleared | 112/238 | 383 | 376 | 45 | 0 | 0 | 0 | 1.5 |
| core-042 | Selective Breeding | easy | base | cleared | 96/175 | 375 | 375 | 29 | 0 | 0 | 0 | -2.1 |
| core-042 | Selective Breeding | hard | base | cleared | 108/231 | 413 | 413 | 26 | 0 | 0 | 0 | -1.9 |
| core-048 | 5pek Controller | easy | base | cleared | 105/206 | 265 | 265 | 69 | 0 | 0 | 0 | 6.6 |
| core-048 | 5pek Controller | hard | base | cleared | 113/244 | 317 | 317 | 49 | 0 | 0 | 0 | 6.4 |
| core-057 | Conjure KY | easy | base | cleared | 116/221 | 313 | 313 | 78 | 0 | 0 | 0 | 2.8 |
| core-057 | Conjure KY | hard | base | cleared | 149/345 | 433 | 433 | 64 | 0 | 0 | 0 | 2.9 |
| core-058 | Rush Token Farm | easy | base | cleared | 78/153 | 133 | 131 | 63 | 0 | 0 | 0 | 1.2 |
| core-058 | Rush Token Farm | hard | base | cleared | 110/252 | 193 | 184 | 71 | 0 | 0 | 0 | 1.1 |
| core-059 | Unbiased Immigration | easy | base | cleared | 91/162 | 223 | 222 | 63 | 0 | 0 | 0 | 1.6 |
| core-059 | Unbiased Immigration | hard | base | cleared | 111/266 | 352 | 339 | 41 | 0 | 0 | 0 | 1.5 |
| core-065 | Masochism Mask | easy | base | cleared | 240/240 | 905 | 900 | 179 | 0 | 0 | 0 | 0.6 |
| core-065 | Masochism Mask | hard | base | cleared | 218/218 | 892 | 866 | 64 | 0 | 0 | 0 | 1.3 |
| core-069 | Call to Arms | easy | base | cleared | 93/176 | 164 | 164 | 62 | 0 | 0 | 0 | 13.1 |
| core-069 | Call to Arms | hard | base | cleared | 114/234 | 219 | 219 | 80 | 0 | 0 | 0 | 12.2 |
| core-072 | Reminisce | easy | base | cleared | 97/201 | 260 | 260 | 82 | 0 | 0 | 0 | -1.3 |
| core-072 | Reminisce | hard | base | cleared | 116/242 | 330 | 330 | 66 | 0 | 0 | 0 | -1.2 |
| core-079 | Twinspell | easy | base | cleared | 104/214 | 286 | 280 | 75 | 0 | 0 | 0 | 1.1 |
| core-079 | Twinspell | hard | base | cleared | 113/228 | 299 | 289 | 64 | 0 | 0 | 0 | 1.0 |
| core-083 | Transmogulate | easy | base | cleared | 95/180 | 337 | 337 | 60 | 0 | 0 | 0 | 22.9 |
| core-083 | Transmogulate | hard | base | cleared | 119/249 | 372 | 372 | 59 | 0 | 0 | 0 | 16.7 |
| core-086 | "Miss" Mrow | easy | base | cleared | 276/514 | 750 | 615 | 193 | 0 | 0 | 0 | 0.5 |
| core-086 | "Miss" Mrow | hard | base | cleared | 258/524 | 731 | 563 | 100 | 0 | 0 | 0 | 1.1 |
| core-087 | Pocket Chaos | easy | base | cleared | 70/129 | 221 | 221 | 44 | 0 | 0 | 0 | 72.6 |
| core-087 | Pocket Chaos | hard | base | cleared | 122/250 | 386 | 386 | 36 | 0 | 0 | 0 | 98.3 |
| core-090 | Super-Virus Injection | easy | base | cleared | 134/252 | 576 | 576 | 58 | 0 | 0 | 0 | -1.4 |
| core-090 | Super-Virus Injection | hard | base | cleared | 138/279 | 460 | 460 | 40 | 0 | 0 | 0 | -0.4 |
| core-092 | Felinor Fiender | easy | base | cleared | 212/369 | 242 | 242 | 195 | 0 | 0 | 0 | 14.5 |
| core-092 | Felinor Fiender | hard | base | cleared | 247/518 | 344 | 344 | 218 | 0 | 0 | 0 | 14.4 |
| core-093 | Combo-Index | easy | base | cleared | 94/193 | 424 | 391 | 84 | 0 | 0 | 0 | 3.9 |
| core-093 | Combo-Index | hard | base | cleared | 160/320 | 552 | 534 | 95 | 0 | 0 | 0 | 3.5 |
| core-094 | Genn's Greed | easy | base | cleared | 82/144 | 179 | 180 | 33 | 0 | 0 | 0 | 2.9 |
| core-094 | Genn's Greed | hard | base | cleared | 141/310 | 434 | 434 | 14 | 0 | 0 | 0 | 0.7 |
| core-099 | Craft a Card | easy | base | cleared | 83/142 | 195 | 196 | 19 | 0 | 0 | 0 | -2.2 |
| core-099 | Craft a Card | hard | base | cleared | 140/296 | 376 | 376 | 13 | 0 | 0 | 0 | -2.7 |
| infiniteassault-005 | Generated Filler | easy | base | cleared | 130/247 | 367 | 367 | 86 | 0 | 0 | 0 | -0.2 |
| infiniteassault-005 | Generated Filler | easy | radiant | cleared | 15/24 | 20 | 20 | 15 | 0 | 0 | 14 | -0.5 |
| infiniteassault-005 | Generated Filler | hard | base | cleared | 120/267 | 355 | 355 | 49 | 0 | 0 | 0 | 0.3 |
| infiniteassault-005 | Generated Filler | hard | radiant | cleared | 12/24 | 18 | 18 | 7 | 0 | 0 | 0 | 0.3 |
| infiniteassault-010 | Assault Carrier | easy | base | cleared | 154/292 | 186 | 142 | 102 | 0 | 0 | 0 | 16.1 |
| infiniteassault-010 | Assault Carrier | easy | radiant | cleared | 16/24 | 16 | 16 | 15 | 0 | 0 | 0 | 26.9 |
| infiniteassault-010 | Assault Carrier | hard | base | cleared | 257/562 | 369 | 296 | 185 | 0 | 0 | 5 | 18.8 |
| infiniteassault-010 | Assault Carrier | hard | radiant | cleared | 11/24 | 17 | 11 | 10 | 0 | 0 | 0 | 28.5 |
| infiniteassault-011 | Quickgen Turret | easy | base | cleared | 200/384 | 409 | 333 | 156 | 0 | 0 | 4 | 7.2 |
| infiniteassault-011 | Quickgen Turret | easy | radiant | cleared | 13/24 | 14 | 13 | 10 | 0 | 0 | 1 | 12.4 |
| infiniteassault-011 | Quickgen Turret | hard | base | cleared | 235/530 | 527 | 416 | 147 | 0 | 0 | 1 | 7.8 |
| infiniteassault-011 | Quickgen Turret | hard | radiant | cleared | 10/24 | 12 | 12 | 9 | 0 | 0 | 18 | 11.6 |
| infiniteassault-012 | Recompile | easy | base | cleared | 100/225 | 232 | 232 | 82 | 0 | 0 | 0 | 6.6 |
| infiniteassault-012 | Recompile | easy | radiant | cleared | 14/24 | 22 | 22 | 9 | 0 | 0 | 0 | 11.4 |
| infiniteassault-012 | Recompile | hard | base | cleared | 123/262 | 255 | 255 | 98 | 0 | 0 | 5 | 8.5 |
| infiniteassault-012 | Recompile | hard | radiant | cleared | 8/24 | 10 | 10 | 7 | 0 | 0 | 0 | 7.0 |
| infiniteassault-013 | Training Run | easy | base | cleared | 100/176 | 314 | 314 | 55 | 0 | 0 | 0 | -1.4 |
| infiniteassault-013 | Training Run | easy | radiant | cleared | 12/24 | 21 | 21 | 11 | 0 | 0 | 1 | -1.9 |
| infiniteassault-013 | Training Run | hard | base | cleared | 128/273 | 432 | 432 | 55 | 0 | 0 | 0 | -1.5 |
| infiniteassault-013 | Training Run | hard | radiant | cleared | 9/24 | 22 | 22 | 4 | 0 | 0 | 1 | -2.1 |
| infiniteassault-015 | Drone Foundry | easy | base | cleared | 113/212 | 270 | 269 | 87 | 0 | 0 | 6 | 0.9 |
| infiniteassault-015 | Drone Foundry | easy | radiant | cleared | 15/24 | 22 | 22 | 10 | 0 | 0 | 0 | 0.8 |
| infiniteassault-015 | Drone Foundry | hard | base | cleared | 111/273 | 335 | 326 | 53 | 0 | 0 | 0 | 1.5 |
| infiniteassault-015 | Drone Foundry | hard | radiant | cleared | 8/24 | 15 | 14 | 4 | 0 | 0 | 0 | 0.6 |
| infiniteassault-017 | Cluster Drone | easy | base | cleared | 209/405 | 389 | 326 | 168 | 0 | 0 | 0 | 5.7 |
| infiniteassault-017 | Cluster Drone | easy | radiant | cleared | 17/24 | 24 | 21 | 16 | 0 | 0 | 0 | 9.9 |
| infiniteassault-017 | Cluster Drone | hard | base | cleared | 268/569 | 658 | 535 | 144 | 0 | 0 | 1 | 5.6 |
| infiniteassault-017 | Cluster Drone | hard | radiant | cleared | 12/24 | 29 | 22 | 8 | 0 | 0 | 1 | 10.7 |
| infiniteassault-018 | Prompt Engineer | easy | base | cleared | 219/396 | 592 | 517 | 146 | 0 | 0 | 5 | 5.9 |
| infiniteassault-018 | Prompt Engineer | easy | radiant | cleared | 13/24 | 27 | 27 | 12 | 0 | 0 | 0 | 11.3 |
| infiniteassault-018 | Prompt Engineer | hard | base | cleared | 257/539 | 752 | 597 | 108 | 0 | 0 | 30 | 6.5 |
| infiniteassault-018 | Prompt Engineer | hard | radiant | cleared | 14/24 | 39 | 32 | 7 | 0 | 0 | 0 | 10.5 |
| infiniteassault-019 | Infinite Scroll | easy | base | cleared | 104/209 | 393 | 394 | 48 | 0 | 0 | 0 | 3.4 |
| infiniteassault-019 | Infinite Scroll | easy | radiant | cleared | 13/24 | 75 | 75 | 8 | 0 | 0 | 1 | 5.9 |
| infiniteassault-019 | Infinite Scroll | hard | base | cleared | 158/347 | 560 | 560 | 33 | 0 | 0 | 2 | 3.2 |
| infiniteassault-019 | Infinite Scroll | hard | radiant | cleared | 14/24 | 42 | 42 | 3 | 0 | 0 | 0 | 3.4 |
| infiniteassault-020 | Synthetic Data | easy | base | cleared | 106/205 | 302 | 302 | 69 | 0 | 0 | 0 | 3.8 |
| infiniteassault-020 | Synthetic Data | easy | radiant | cleared | 11/24 | 24 | 24 | 9 | 0 | 0 | 9 | 4.6 |
| infiniteassault-020 | Synthetic Data | hard | base | cleared | 122/258 | 455 | 455 | 33 | 0 | 0 | 0 | 0.6 |
| infiniteassault-020 | Synthetic Data | hard | radiant | cleared | 11/24 | 42 | 42 | 6 | 0 | 0 | 13 | 1.7 |
| infiniteassault-021 | Endless Production Line | easy | base | cleared | 76/173 | 144 | 135 | 37 | 0 | 0 | 0 | 2.9 |
| infiniteassault-021 | Endless Production Line | easy | radiant | cleared | 14/24 | 19 | 19 | 9 | 0 | 0 | 2 | 4.2 |
| infiniteassault-021 | Endless Production Line | hard | base | cleared | 162/316 | 428 | 409 | 54 | 0 | 0 | 0 | 3.2 |
| infiniteassault-021 | Endless Production Line | hard | radiant | cleared | 14/24 | 26 | 26 | 7 | 0 | 0 | 0 | 4.3 |
| infiniteassault-022 | Assault Mainframe | easy | base | cleared | 177/356 | 197 | 166 | 114 | 0 | 0 | 7 | 37.4 |
| infiniteassault-022 | Assault Mainframe | easy | radiant | cleared | 15/24 | 14 | 12 | 13 | 0 | 0 | 0 | 44.2 |
| infiniteassault-022 | Assault Mainframe | hard | base | cleared | 256/537 | 394 | 298 | 180 | 0 | 0 | 2 | 26.7 |
| infiniteassault-022 | Assault Mainframe | hard | radiant | cleared | 15/24 | 18 | 17 | 11 | 0 | 0 | 5 | 49.1 |
| infiniteassault-023 | The Generator | easy | base | cleared | 228/413 | 462 | 380 | 164 | 0 | 0 | 16 | 10.7 |
| infiniteassault-023 | The Generator | easy | radiant | cleared | 16/24 | 20 | 18 | 14 | 0 | 0 | 2 | 16.7 |
| infiniteassault-023 | The Generator | hard | base | cleared | 281/646 | 568 | 442 | 166 | 0 | 0 | 0 | 11.6 |
| infiniteassault-023 | The Generator | hard | radiant | cleared | 6/24 | 12 | 9 | 2 | 0 | 0 | 2 | 17.8 |
| infiniteassault-024 | Mass Deployment | easy | base | cleared | 87/144 | 128 | 128 | 47 | 0 | 0 | 1 | 20.2 |
| infiniteassault-024 | Mass Deployment | easy | radiant | cleared | 14/24 | 14 | 14 | 8 | 0 | 0 | 0 | 53.8 |
| infiniteassault-024 | Mass Deployment | hard | base | cleared | 147/296 | 273 | 275 | 79 | 0 | 0 | 1 | 22.7 |
| infiniteassault-024 | Mass Deployment | hard | radiant | cleared | 6/24 | 10 | 10 | 6 | 0 | 0 | 0 | 58.1 |
| infiniteassault-027 | Decoy Projector | easy | base | cleared | 231/424 | 413 | 358 | 187 | 0 | 0 | 0 | 14.9 |
| infiniteassault-027 | Decoy Projector | easy | radiant | cleared | 11/24 | 23 | 19 | 10 | 0 | 0 | 0 | 29.8 |
| infiniteassault-027 | Decoy Projector | hard | base | cleared | 259/542 | 493 | 400 | 169 | 0 | 0 | 2 | 11.8 |
| infiniteassault-027 | Decoy Projector | hard | radiant | cleared | 7/24 | 7 | 7 | 7 | 0 | 0 | 0 | 13.4 |
| infiniteassault-029 | False Positive | easy | base | timeout | 13/24 | 28 | 24 | 10 | 0 | 0 | 0 | 0.5 |
| infiniteassault-029 | False Positive | easy | radiant | cleared | 14/24 | 40 | 40 | 13 | 0 | 0 | 0 | 0.3 |
| infiniteassault-029 | False Positive | hard | base | timeout | 13/24 | 28 | 28 | 9 | 0 | 0 | 0 | 0.9 |
| infiniteassault-029 | False Positive | hard | radiant | cleared | 11/24 | 17 | 16 | 7 | 0 | 0 | 0 | -0.3 |
| infiniteassault-034 | Mass Hallucination | easy | base | cleared | 110/222 | 341 | 341 | 64 | 0 | 0 | 0 | 17.7 |
| infiniteassault-034 | Mass Hallucination | easy | radiant | cleared | 10/24 | 37 | 37 | 9 | 0 | 0 | 0 | 5.2 |
| infiniteassault-034 | Mass Hallucination | hard | base | cleared | 136/300 | 428 | 428 | 49 | 0 | 0 | 0 | 7.3 |
| infiniteassault-034 | Mass Hallucination | hard | radiant | cleared | 11/24 | 35 | 35 | 3 | 0 | 0 | 0 | 7.6 |
| infiniteassault-035 | Fata Morgana | easy | base | error | 206/414 | 323 | 268 | 157 | 1 | 0 | 1 | 8.9 |
| infiniteassault-035 | Fata Morgana | easy | radiant | cleared | 10/24 | 7 | 7 | 9 | 0 | 0 | 39 | 13.5 |
| infiniteassault-035 | Fata Morgana | hard | base | error | 225/559 | 463 | 358 | 139 | 0 | 0 | 0 | 8.8 |
| infiniteassault-035 | Fata Morgana | hard | radiant | cleared | 12/24 | 13 | 10 | 8 | 0 | 0 | 22 | 13.4 |
| infiniteassault-037 | Bluff Called | easy | base | cleared | 110/220 | 386 | 381 | 65 | 0 | 0 | 2 | 1.5 |
| infiniteassault-037 | Bluff Called | easy | radiant | cleared | 11/24 | 39 | 38 | 5 | 0 | 0 | 0 | 0.7 |
| infiniteassault-037 | Bluff Called | hard | base | cleared | 127/273 | 426 | 419 | 47 | 0 | 0 | 5 | 1.7 |
| infiniteassault-037 | Bluff Called | hard | radiant | cleared | 11/24 | 45 | 45 | 5 | 0 | 0 | 1 | 1.0 |
| infiniteassault-039 | Shell Game | easy | base | cleared | 143/301 | 271 | 271 | 111 | 0 | 0 | 6 | 2.8 |
| infiniteassault-039 | Shell Game | easy | radiant | cleared | 14/24 | 33 | 33 | 11 | 0 | 0 | 4 | 2.6 |
| infiniteassault-039 | Shell Game | hard | base | cleared | 152/340 | 296 | 296 | 121 | 0 | 0 | 1 | 2.9 |
| infiniteassault-039 | Shell Game | hard | radiant | cleared | 10/24 | 31 | 31 | 6 | 0 | 0 | 0 | 3.4 |
| infiniteassault-040 | Hall of Mirrors | easy | base | cleared | 125/264 | 466 | 454 | 42 | 0 | 0 | 1 | 1.8 |
| infiniteassault-040 | Hall of Mirrors | easy | radiant | cleared | 14/24 | 68 | 66 | 8 | 0 | 0 | 0 | 1.4 |
| infiniteassault-040 | Hall of Mirrors | hard | base | cleared | 188/431 | 614 | 597 | 52 | 0 | 0 | 12 | 1.8 |
| infiniteassault-040 | Hall of Mirrors | hard | radiant | cleared | 8/24 | 35 | 31 | 1 | 0 | 0 | 1 | 1.5 |
| infiniteassault-041 | Phantom Army | easy | base | cleared | 125/235 | 389 | 392 | 65 | 0 | 0 | 2 | 5.3 |
| infiniteassault-041 | Phantom Army | easy | radiant | cleared | 15/24 | 36 | 36 | 11 | 0 | 0 | 4 | 13.6 |
| infiniteassault-041 | Phantom Army | hard | base | cleared | 200/401 | 611 | 611 | 52 | 0 | 0 | 7 | 5.0 |
| infiniteassault-041 | Phantom Army | hard | radiant | cleared | 9/24 | 24 | 24 | 3 | 0 | 0 | 0 | 11.9 |
| infiniteassault-042 | Paranoia Engine | easy | base | cleared | 251/476 | 421 | 366 | 205 | 0 | 0 | 4 | 7.9 |
| infiniteassault-042 | Paranoia Engine | easy | radiant | cleared | 10/24 | 10 | 6 | 8 | 0 | 0 | 1 | 11.3 |
| infiniteassault-042 | Paranoia Engine | hard | base | cleared | 269/577 | 486 | 378 | 190 | 0 | 0 | 1 | 7.8 |
| infiniteassault-042 | Paranoia Engine | hard | radiant | cleared | 11/24 | 13 | 12 | 10 | 0 | 0 | 0 | 15.6 |
| infiniteassault-043 | Cognitive Dissonance | easy | base | cleared | 119/222 | 428 | 414 | 64 | 0 | 0 | 3 | 1.1 |
| infiniteassault-043 | Cognitive Dissonance | easy | radiant | cleared | 12/24 | 42 | 41 | 7 | 0 | 0 | 3 | 0.1 |
| infiniteassault-043 | Cognitive Dissonance | hard | base | cleared | 149/332 | 512 | 485 | 53 | 0 | 0 | 9 | 1.2 |
| infiniteassault-043 | Cognitive Dissonance | hard | radiant | cleared | 13/24 | 62 | 60 | 3 | 0 | 0 | 1 | 1.0 |
| infiniteassault-045 | Dream Within a Dream | easy | base | cleared | 119/220 | 236 | 236 | 52 | 0 | 0 | 1 | 23.5 |
| infiniteassault-045 | Dream Within a Dream | easy | radiant | cleared | 13/24 | 20 | 20 | 8 | 0 | 0 | 1 | 36.4 |
| infiniteassault-045 | Dream Within a Dream | hard | base | cleared | 192/395 | 569 | 569 | 59 | 0 | 0 | 7 | 12.0 |
| infiniteassault-045 | Dream Within a Dream | hard | radiant | cleared | 11/24 | 29 | 29 | 4 | 0 | 0 | 0 | 25.2 |
| infiniteassault-047 | Unreal Estate | easy | base | cleared | 110/203 | 308 | 302 | 59 | 0 | 0 | 0 | 1.5 |
| infiniteassault-047 | Unreal Estate | easy | radiant | cleared | 12/24 | 36 | 36 | 3 | 0 | 0 | 1 | 0.7 |
| infiniteassault-047 | Unreal Estate | hard | base | cleared | 155/297 | 464 | 443 | 63 | 0 | 0 | 0 | 2.0 |
| infiniteassault-047 | Unreal Estate | hard | radiant | cleared | 12/24 | 45 | 43 | 1 | 0 | 0 | 3 | 1.0 |
| infiniteassault-048 | Ego Death | easy | base | timeout | 157/272 | 569 | 572 | 69 | 0 | 5 | 17 | 8.2 |
| infiniteassault-048 | Ego Death | easy | radiant | neverPlayed | 16/24 | 57 | 57 | 7 | 0 | 0 | 0 | 9.1 |
| infiniteassault-048 | Ego Death | hard | base | timeout | 195/394 | 624 | 624 | 23 | 0 | 0 | 0 | 6.7 |
| infiniteassault-048 | Ego Death | hard | radiant | neverPlayed | 12/24 | 44 | 44 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-050 | Burst Clock | easy | base | cleared | 169/296 | 399 | 399 | 103 | 0 | 0 | 0 | -2.1 |
| infiniteassault-050 | Burst Clock | easy | radiant | cleared | 13/24 | 22 | 22 | 13 | 0 | 0 | 0 | -2.3 |
| infiniteassault-050 | Burst Clock | hard | base | cleared | 134/306 | 295 | 295 | 72 | 0 | 0 | 9 | -1.7 |
| infiniteassault-050 | Burst Clock | hard | radiant | cleared | 13/24 | 20 | 20 | 6 | 0 | 0 | 0 | -2.8 |
| infiniteassault-052 | Thermal Paste | easy | base | cleared | 145/293 | 513 | 513 | 100 | 0 | 0 | 0 | -4.2 |
| infiniteassault-052 | Thermal Paste | easy | radiant | cleared | 10/24 | 31 | 31 | 6 | 0 | 0 | 3 | -5.0 |
| infiniteassault-052 | Thermal Paste | hard | base | cleared | 105/288 | 338 | 338 | 53 | 0 | 0 | 0 | -3.7 |
| infiniteassault-052 | Thermal Paste | hard | radiant | cleared | 15/24 | 45 | 45 | 10 | 0 | 0 | 0 | -4.5 |
| infiniteassault-053 | Overvolt Grunt | easy | base | cleared | 261/487 | 392 | 345 | 225 | 0 | 0 | 1 | 7.1 |
| infiniteassault-053 | Overvolt Grunt | easy | radiant | cleared | 13/24 | 14 | 14 | 11 | 0 | 0 | 0 | 14.5 |
| infiniteassault-053 | Overvolt Grunt | hard | base | cleared | 237/540 | 382 | 309 | 163 | 0 | 0 | 6 | 8.0 |
| infiniteassault-053 | Overvolt Grunt | hard | radiant | cleared | 8/24 | 9 | 5 | 6 | 0 | 0 | 0 | 12.7 |
| infiniteassault-056 | Afterburner | easy | base | cleared | 129/229 | 316 | 316 | 94 | 0 | 0 | 0 | 3.9 |
| infiniteassault-056 | Afterburner | easy | radiant | cleared | 11/24 | 13 | 13 | 10 | 0 | 0 | 4 | 9.7 |
| infiniteassault-056 | Afterburner | hard | base | cleared | 136/322 | 356 | 356 | 92 | 0 | 0 | 0 | 5.7 |
| infiniteassault-056 | Afterburner | hard | radiant | cleared | 13/24 | 21 | 21 | 13 | 0 | 0 | 0 | 12.9 |
| infiniteassault-058 | Overclocked Core | easy | base | cleared | 193/394 | 503 | 420 | 141 | 0 | 0 | 0 | 5.6 |
| infiniteassault-058 | Overclocked Core | easy | radiant | cleared | 14/24 | 23 | 18 | 14 | 0 | 0 | 9 | 13.6 |
| infiniteassault-058 | Overclocked Core | hard | base | cleared | 287/591 | 841 | 648 | 109 | 0 | 0 | 5 | 6.0 |
| infiniteassault-058 | Overclocked Core | hard | radiant | cleared | 11/24 | 17 | 16 | 11 | 0 | 0 | 0 | 15.9 |
| infiniteassault-061 | Meltdown Charge | easy | base | cleared | 123/251 | 401 | 401 | 74 | 0 | 0 | 0 | 20.0 |
| infiniteassault-061 | Meltdown Charge | easy | radiant | cleared | 9/24 | 20 | 20 | 9 | 0 | 0 | 0 | 15.4 |
| infiniteassault-061 | Meltdown Charge | hard | base | cleared | 186/387 | 494 | 495 | 104 | 0 | 0 | 1 | 19.4 |
| infiniteassault-061 | Meltdown Charge | hard | radiant | cleared | 11/24 | 23 | 23 | 9 | 0 | 0 | 0 | 38.7 |
| infiniteassault-062 | Burnout | easy | base | cleared | 150/274 | 286 | 275 | 129 | 0 | 0 | 0 | 0.8 |
| infiniteassault-062 | Burnout | easy | radiant | cleared | 11/24 | 20 | 19 | 8 | 0 | 0 | 0 | 0.1 |
| infiniteassault-062 | Burnout | hard | base | cleared | 134/294 | 237 | 231 | 98 | 0 | 0 | 0 | 0.7 |
| infiniteassault-062 | Burnout | hard | radiant | cleared | 13/24 | 26 | 26 | 6 | 0 | 0 | 0 | 0.2 |
| infiniteassault-065 | Overdrive | easy | base | cleared | 128/248 | 502 | 503 | 35 | 0 | 0 | 0 | -1.7 |
| infiniteassault-065 | Overdrive | easy | radiant | cleared | 12/24 | 55 | 55 | 4 | 0 | 0 | 9 | -2.4 |
| infiniteassault-065 | Overdrive | hard | base | cleared | 187/408 | 595 | 595 | 32 | 0 | 0 | 0 | -1.4 |
| infiniteassault-065 | Overdrive | hard | radiant | cleared | 12/24 | 45 | 45 | 2 | 0 | 0 | 0 | -2.4 |
| infiniteassault-067 | Burning Ambition | easy | base | cleared | 113/218 | 359 | 339 | 57 | 0 | 0 | 0 | 1.5 |
| infiniteassault-067 | Burning Ambition | easy | radiant | cleared | 18/24 | 61 | 61 | 6 | 0 | 0 | 1 | 0.5 |
| infiniteassault-067 | Burning Ambition | hard | base | cleared | 125/282 | 451 | 439 | 36 | 0 | 0 | 3 | 1.4 |
| infiniteassault-067 | Burning Ambition | hard | radiant | cleared | 12/24 | 42 | 36 | 2 | 0 | 0 | 8 | 0.5 |
| infiniteassault-068 | Short Circuit | easy | base | cleared | 106/199 | 252 | 243 | 73 | 0 | 0 | 0 | 1.5 |
| infiniteassault-068 | Short Circuit | easy | radiant | cleared | 9/24 | 19 | 19 | 6 | 0 | 0 | 0 | 0.7 |
| infiniteassault-068 | Short Circuit | hard | base | cleared | 124/308 | 401 | 397 | 50 | 0 | 0 | 1 | 0.5 |
| infiniteassault-068 | Short Circuit | hard | radiant | cleared | 7/24 | 25 | 25 | 4 | 0 | 0 | 0 | 1.0 |
| infiniteassault-071 | Thermal Throttler | easy | base | cleared | 213/406 | 330 | 283 | 162 | 0 | 0 | 0 | 10.4 |
| infiniteassault-071 | Thermal Throttler | easy | radiant | cleared | 11/24 | 14 | 13 | 9 | 0 | 0 | 0 | 15.8 |
| infiniteassault-071 | Thermal Throttler | hard | base | cleared | 229/547 | 454 | 364 | 141 | 0 | 0 | 0 | 9.5 |
| infiniteassault-071 | Thermal Throttler | hard | radiant | cleared | 9/24 | 12 | 10 | 6 | 0 | 0 | 0 | 15.8 |
| infiniteassault-072 | Overclock to Infinity | easy | base | cleared | 101/198 | 324 | 324 | 9 | 0 | 0 | 0 | -1.7 |
| infiniteassault-072 | Overclock to Infinity | easy | radiant | neverPlayed | 11/24 | 31 | 31 | 1 | 0 | 0 | 1 | -3.2 |
| infiniteassault-072 | Overclock to Infinity | hard | base | cleared | 160/339 | 483 | 482 | 15 | 0 | 0 | 2 | -1.4 |
| infiniteassault-072 | Overclock to Infinity | hard | radiant | neverPlayed | 8/24 | 31 | 30 | 0 | 0 | 0 | 9 | n/a |
| infiniteassault-078 | Batch Job | easy | base | cleared | 115/233 | 277 | 277 | 71 | 0 | 0 | 9 | 11.7 |
| infiniteassault-078 | Batch Job | easy | radiant | cleared | 16/24 | 24 | 24 | 14 | 0 | 0 | 0 | 21.5 |
| infiniteassault-078 | Batch Job | hard | base | cleared | 171/384 | 442 | 441 | 71 | 0 | 0 | 0 | 9.6 |
| infiniteassault-078 | Batch Job | hard | radiant | cleared | 11/24 | 21 | 21 | 7 | 0 | 0 | 6 | 24.4 |
| infiniteassault-079 | Thread Pool | easy | base | cleared | 218/431 | 480 | 383 | 145 | 0 | 0 | 0 | 13.0 |
| infiniteassault-079 | Thread Pool | easy | radiant | cleared | 10/24 | 17 | 17 | 8 | 0 | 0 | 0 | 19.4 |
| infiniteassault-079 | Thread Pool | hard | base | cleared | 325/708 | 783 | 585 | 159 | 0 | 0 | 1 | 14.0 |
| infiniteassault-079 | Thread Pool | hard | radiant | cleared | 11/24 | 24 | 19 | 9 | 0 | 0 | 0 | 21.0 |
| infiniteassault-084 | Distributed Mind | easy | base | cleared | 214/435 | 600 | 467 | 120 | 0 | 0 | 0 | 9.7 |
| infiniteassault-084 | Distributed Mind | easy | radiant | cleared | 13/24 | 24 | 24 | 12 | 0 | 0 | 0 | 10.7 |
| infiniteassault-084 | Distributed Mind | hard | base | cleared | 292/646 | 853 | 623 | 107 | 0 | 0 | 0 | 8.5 |
| infiniteassault-084 | Distributed Mind | hard | radiant | cleared | 13/24 | 31 | 14 | 11 | 0 | 0 | 9 | 15.1 |
| infiniteassault-085 | Merge Conflict | easy | base | cleared | 142/230 | 629 | 339 | 73 | 0 | 0 | 15 | 6.6 |
| infiniteassault-085 | Merge Conflict | easy | radiant | cleared | 13/24 | 28 | 17 | 10 | 0 | 0 | 0 | 0.6 |
| infiniteassault-085 | Merge Conflict | hard | base | cleared | 149/294 | 521 | 330 | 75 | 0 | 0 | 0 | 4.0 |
| infiniteassault-085 | Merge Conflict | hard | radiant | cleared | 11/24 | 45 | 26 | 3 | 0 | 0 | 1 | 6.7 |
| infiniteassault-086 | Replica Hatchery | easy | base | cleared | 107/194 | 328 | 323 | 63 | 0 | 0 | 1 | 1.7 |
| infiniteassault-086 | Replica Hatchery | easy | radiant | cleared | 11/24 | 24 | 24 | 8 | 0 | 0 | 0 | 1.0 |
| infiniteassault-086 | Replica Hatchery | hard | base | cleared | 130/291 | 381 | 366 | 74 | 0 | 0 | 0 | 1.4 |
| infiniteassault-086 | Replica Hatchery | hard | radiant | cleared | 14/24 | 31 | 30 | 6 | 0 | 0 | 1 | 1.0 |
| infiniteassault-088 | Scale Out | easy | base | cleared | 135/236 | 398 | 398 | 91 | 0 | 0 | 33 | 10.1 |
| infiniteassault-088 | Scale Out | easy | radiant | cleared | 12/24 | 22 | 22 | 8 | 0 | 0 | 0 | 13.4 |
| infiniteassault-088 | Scale Out | hard | base | cleared | 135/292 | 459 | 459 | 53 | 0 | 0 | 0 | 8.5 |
| infiniteassault-088 | Scale Out | hard | radiant | cleared | 13/24 | 30 | 30 | 9 | 0 | 0 | 4 | 19.4 |
| infiniteassault-090 | Mass Production | easy | base | cleared | 108/200 | 206 | 206 | 67 | 0 | 0 | 0 | 22.0 |
| infiniteassault-090 | Mass Production | easy | radiant | cleared | 10/24 | 11 | 11 | 6 | 0 | 0 | 0 | 21.4 |
| infiniteassault-090 | Mass Production | hard | base | cleared | 189/371 | 377 | 377 | 115 | 0 | 0 | 6 | 24.8 |
| infiniteassault-090 | Mass Production | hard | radiant | cleared | 12/24 | 20 | 20 | 9 | 0 | 0 | 0 | 27.3 |
| infiniteassault-091 | Flood the Zone | easy | base | cleared | 137/254 | 381 | 381 | 79 | 0 | 0 | 2 | 7.8 |
| infiniteassault-091 | Flood the Zone | easy | radiant | cleared | 16/24 | 30 | 30 | 15 | 0 | 0 | 13 | 18.1 |
| infiniteassault-091 | Flood the Zone | hard | base | cleared | 199/395 | 546 | 546 | 78 | 0 | 0 | 2 | 9.2 |
| infiniteassault-091 | Flood the Zone | hard | radiant | cleared | 12/24 | 19 | 19 | 8 | 0 | 0 | 1 | 17.2 |
| infiniteassault-092 | Hive Overseer | easy | base | cleared | 244/468 | 471 | 382 | 173 | 0 | 0 | 2 | 20.1 |
| infiniteassault-092 | Hive Overseer | easy | radiant | cleared | 13/24 | 18 | 18 | 13 | 0 | 0 | 0 | 29.9 |
| infiniteassault-092 | Hive Overseer | hard | base | cleared | 297/626 | 622 | 492 | 170 | 0 | 0 | 1 | 14.9 |
| infiniteassault-092 | Hive Overseer | hard | radiant | cleared | 13/24 | 22 | 20 | 10 | 0 | 0 | 1 | 16.3 |
| infiniteassault-093 | Gestalt | easy | base | cleared | 203/361 | 538 | 428 | 44 | 0 | 0 | 2 | 4.8 |
| infiniteassault-093 | Gestalt | easy | radiant | cleared | 10/24 | 11 | 8 | 8 | 0 | 0 | 0 | 32.5 |
| infiniteassault-093 | Gestalt | hard | base | cleared | 274/598 | 855 | 592 | 52 | 0 | 0 | 0 | 6.0 |
| infiniteassault-093 | Gestalt | hard | radiant | cleared | 15/24 | 29 | 25 | 9 | 0 | 0 | 3 | 31.8 |
| infiniteassault-095 | The Botnet | easy | base | cleared | 108/193 | 159 | 154 | 75 | 0 | 0 | 0 | 29.8 |
| infiniteassault-095 | The Botnet | easy | radiant | cleared | 15/24 | 27 | 27 | 9 | 0 | 0 | 2 | 37.5 |
| infiniteassault-095 | The Botnet | hard | base | cleared | 176/385 | 311 | 299 | 135 | 0 | 0 | 0 | 32.5 |
| infiniteassault-095 | The Botnet | hard | radiant | cleared | 12/24 | 20 | 18 | 9 | 0 | 0 | 1 | 35.3 |
| infiniteassault-098 | Echo Chamber | easy | base | cleared | 157/289 | 397 | 397 | 133 | 0 | 0 | 2 | 3.0 |
| infiniteassault-098 | Echo Chamber | easy | radiant | cleared | 13/24 | 29 | 29 | 13 | 0 | 0 | 2 | 1.5 |
| infiniteassault-098 | Echo Chamber | hard | base | cleared | 146/307 | 327 | 327 | 90 | 0 | 0 | 0 | 2.4 |
| infiniteassault-098 | Echo Chamber | hard | radiant | cleared | 7/24 | 7 | 7 | 3 | 0 | 0 | 4 | 3.7 |
| infiniteassault-099 | Recall | easy | base | cleared | 144/267 | 523 | 523 | 91 | 0 | 0 | 4 | 0.3 |
| infiniteassault-099 | Recall | easy | radiant | cleared | 12/24 | 39 | 39 | 7 | 0 | 0 | 3 | 1.1 |
| infiniteassault-099 | Recall | hard | base | cleared | 127/281 | 416 | 416 | 59 | 0 | 0 | 3 | 0.4 |
| infiniteassault-099 | Recall | hard | radiant | cleared | 9/24 | 24 | 24 | 5 | 0 | 0 | 0 | 0.4 |
| infiniteassault-100 | Iterator | easy | base | cleared | 227/427 | 615 | 502 | 182 | 0 | 0 | 0 | 7.3 |
| infiniteassault-100 | Iterator | easy | radiant | cleared | 13/24 | 17 | 14 | 12 | 0 | 0 | 4 | 11.2 |
| infiniteassault-100 | Iterator | hard | base | cleared | 283/570 | 769 | 610 | 129 | 0 | 0 | 5 | 7.3 |
| infiniteassault-100 | Iterator | hard | radiant | cleared | 9/24 | 13 | 13 | 8 | 0 | 0 | 3 | 10.9 |
| infiniteassault-103 | Undo | easy | base | cleared | 124/238 | 305 | 305 | 104 | 0 | 0 | 0 | 17.0 |
| infiniteassault-103 | Undo | easy | radiant | cleared | 14/24 | 30 | 30 | 14 | 0 | 0 | 0 | 25.0 |
| infiniteassault-103 | Undo | hard | base | cleared | 133/304 | 376 | 375 | 93 | 0 | 0 | 6 | 15.5 |
| infiniteassault-103 | Undo | hard | radiant | cleared | 12/24 | 25 | 25 | 11 | 0 | 0 | 4 | 9.8 |
| infiniteassault-105 | Callback | easy | base | cleared | 226/425 | 459 | 371 | 168 | 0 | 0 | 0 | 8.1 |
| infiniteassault-105 | Callback | easy | radiant | cleared | 14/24 | 13 | 11 | 13 | 0 | 0 | 0 | 12.6 |
| infiniteassault-105 | Callback | hard | base | cleared | 244/558 | 576 | 469 | 142 | 0 | 0 | 0 | 8.3 |
| infiniteassault-105 | Callback | hard | radiant | cleared | 6/24 | 8 | 8 | 6 | 0 | 0 | 4 | 15.9 |
| infiniteassault-106 | Checkpoint Daemon | easy | base | cleared | 244/455 | 706 | 600 | 123 | 0 | 0 | 1 | 7.5 |
| infiniteassault-106 | Checkpoint Daemon | easy | radiant | cleared | 15/24 | 27 | 24 | 11 | 0 | 0 | 19 | 11.1 |
| infiniteassault-106 | Checkpoint Daemon | hard | base | cleared | 317/649 | 962 | 727 | 97 | 0 | 0 | 0 | 7.8 |
| infiniteassault-106 | Checkpoint Daemon | hard | radiant | cleared | 12/24 | 22 | 19 | 8 | 0 | 0 | 0 | 13.8 |
| infiniteassault-107 | Recursive Call | easy | base | cleared | 104/195 | 478 | 478 | 75 | 0 | 0 | 0 | 3.1 |
| infiniteassault-107 | Recursive Call | easy | radiant | cleared | 12/24 | 41 | 41 | 6 | 0 | 0 | 1 | 3.5 |
| infiniteassault-107 | Recursive Call | hard | base | cleared | 136/304 | 505 | 505 | 52 | 0 | 0 | 7 | 2.5 |
| infiniteassault-107 | Recursive Call | hard | radiant | cleared | 11/24 | 31 | 31 | 8 | 0 | 0 | 0 | 1.0 |
| infiniteassault-108 | Second Draft | easy | base | cleared | 154/293 | 364 | 364 | 110 | 0 | 0 | 0 | 1.2 |
| infiniteassault-108 | Second Draft | easy | radiant | cleared | 13/24 | 25 | 25 | 8 | 0 | 0 | 1 | 1.7 |
| infiniteassault-108 | Second Draft | hard | base | cleared | 122/281 | 314 | 314 | 68 | 0 | 0 | 0 | 0.4 |
| infiniteassault-108 | Second Draft | hard | radiant | cleared | 13/24 | 34 | 34 | 6 | 0 | 0 | 0 | 0.8 |
| infiniteassault-109 | Revenant Process | easy | base | cleared | 215/415 | 363 | 295 | 173 | 0 | 0 | 2 | 9.1 |
| infiniteassault-109 | Revenant Process | easy | radiant | cleared | 17/24 | 26 | 17 | 14 | 0 | 0 | 5 | 13.1 |
| infiniteassault-109 | Revenant Process | hard | base | cleared | 268/577 | 534 | 414 | 160 | 0 | 0 | 2 | 10.7 |
| infiniteassault-109 | Revenant Process | hard | radiant | cleared | 8/24 | 12 | 9 | 6 | 0 | 0 | 0 | 13.4 |
| infiniteassault-110 | Infinite Loop | easy | base | cleared | 112/226 | 370 | 360 | 53 | 0 | 0 | 0 | 1.8 |
| infiniteassault-110 | Infinite Loop | easy | radiant | cleared | 14/24 | 50 | 49 | 7 | 0 | 0 | 26 | 1.4 |
| infiniteassault-110 | Infinite Loop | hard | base | cleared | 171/391 | 562 | 530 | 48 | 0 | 0 | 1 | 1.8 |
| infiniteassault-110 | Infinite Loop | hard | radiant | cleared | 11/24 | 38 | 33 | 5 | 0 | 0 | 0 | 1.7 |
| infiniteassault-111 | Redundant Copy | easy | base | cleared | 139/277 | 278 | 273 | 114 | 0 | 0 | 0 | 0.8 |
| infiniteassault-111 | Redundant Copy | easy | radiant | cleared | 14/24 | 28 | 25 | 12 | 0 | 0 | 1 | -0.2 |
| infiniteassault-111 | Redundant Copy | hard | base | cleared | 124/274 | 282 | 278 | 78 | 0 | 0 | 3 | 0.9 |
| infiniteassault-111 | Redundant Copy | hard | radiant | cleared | 13/24 | 32 | 32 | 9 | 0 | 0 | 17 | 0.1 |
| infiniteassault-112 | Evolutionary Loop | easy | base | cleared | 209/407 | 478 | 405 | 138 | 0 | 0 | 1 | 5.0 |
| infiniteassault-112 | Evolutionary Loop | easy | radiant | cleared | 11/24 | 17 | 15 | 8 | 0 | 0 | 8 | 6.4 |
| infiniteassault-112 | Evolutionary Loop | hard | base | cleared | 238/558 | 643 | 508 | 101 | 0 | 0 | 6 | 5.0 |
| infiniteassault-112 | Evolutionary Loop | hard | radiant | cleared | 3/24 | 3 | 3 | 3 | 0 | 0 | 1 | 11.6 |
| infiniteassault-113 | Rerun | easy | base | cleared | 140/244 | 426 | 426 | 90 | 0 | 0 | 15 | 29.0 |
| infiniteassault-113 | Rerun | easy | radiant | cleared | 13/24 | 46 | 46 | 8 | 0 | 0 | 1 | 11.1 |
| infiniteassault-113 | Rerun | hard | base | cleared | 153/376 | 461 | 461 | 57 | 0 | 0 | 0 | 14.5 |
| infiniteassault-113 | Rerun | hard | radiant | cleared | 14/24 | 60 | 60 | 5 | 0 | 0 | 0 | 6.2 |
| infiniteassault-114 | Ouroboros Engine | easy | base | cleared | 224/435 | 530 | 447 | 174 | 0 | 0 | 4 | 11.6 |
| infiniteassault-114 | Ouroboros Engine | easy | radiant | cleared | 15/24 | 26 | 20 | 17 | 0 | 0 | 0 | 18.5 |
| infiniteassault-114 | Ouroboros Engine | hard | base | cleared | 312/694 | 694 | 546 | 180 | 0 | 0 | 0 | 11.8 |
| infiniteassault-114 | Ouroboros Engine | hard | radiant | cleared | 10/24 | 9 | 6 | 8 | 0 | 0 | 0 | 18.5 |
| infiniteassault-116 | Groundhog Day | easy | base | cleared | 105/187 | 238 | 238 | 31 | 0 | 0 | 1 | 18.4 |
| infiniteassault-116 | Groundhog Day | easy | radiant | cleared | 16/24 | 34 | 34 | 8 | 0 | 0 | 1 | 17.5 |
| infiniteassault-116 | Groundhog Day | hard | base | cleared | 162/351 | 422 | 422 | 41 | 0 | 0 | 1 | 19.1 |
| infiniteassault-116 | Groundhog Day | hard | radiant | cleared | 11/24 | 25 | 25 | 4 | 0 | 0 | 0 | 18.9 |
| infiniteassault-117 | Core Dump | easy | base | cleared | 117/208 | 374 | 358 | 64 | 0 | 0 | 5 | 1.5 |
| infiniteassault-117 | Core Dump | easy | radiant | cleared | 14/24 | 36 | 36 | 8 | 0 | 0 | 1 | 0.9 |
| infiniteassault-117 | Core Dump | hard | base | cleared | 138/290 | 454 | 447 | 58 | 0 | 0 | 0 | 1.3 |
| infiniteassault-117 | Core Dump | hard | radiant | cleared | 14/24 | 49 | 45 | 2 | 0 | 0 | 2 | 0.8 |
| infiniteassault-118 | The Recursionist | easy | base | cleared | 232/422 | 472 | 387 | 152 | 0 | 0 | 2 | 12.1 |
| infiniteassault-118 | The Recursionist | easy | radiant | cleared | 12/24 | 12 | 11 | 10 | 0 | 0 | 13 | 24.2 |
| infiniteassault-118 | The Recursionist | hard | base | cleared | 290/659 | 609 | 466 | 178 | 0 | 0 | 5 | 12.5 |
| infiniteassault-118 | The Recursionist | hard | radiant | cleared | 10/24 | 12 | 9 | 7 | 0 | 0 | 2 | 27.4 |
| infiniteassault-119 | Snapshot Lich | easy | base | cleared | 182/357 | 231 | 200 | 121 | 0 | 0 | 7 | 18.0 |
| infiniteassault-119 | Snapshot Lich | easy | radiant | cleared | 14/24 | 17 | 16 | 14 | 0 | 0 | 1 | 28.1 |
| infiniteassault-119 | Snapshot Lich | hard | base | cleared | 297/621 | 553 | 425 | 184 | 0 | 0 | 28 | 19.0 |
| infiniteassault-119 | Snapshot Lich | hard | radiant | cleared | 8/24 | 16 | 13 | 6 | 0 | 0 | 1 | 28.5 |
| infiniteassault-120 | Eternal Return | easy | base | cleared | 101/194 | 249 | 249 | 38 | 0 | 0 | 2 | 2.7 |
| infiniteassault-120 | Eternal Return | easy | radiant | cleared | 11/24 | 29 | 29 | 4 | 0 | 0 | 0 | 4.9 |
| infiniteassault-120 | Eternal Return | hard | base | cleared | 164/342 | 467 | 467 | 34 | 0 | 0 | 1 | 2.5 |
| infiniteassault-120 | Eternal Return | hard | radiant | cleared | 14/24 | 35 | 35 | 2 | 0 | 0 | 0 | 2.3 |
| infiniteassault-123 | Cost Injection | easy | base | timeout | 11/24 | 48 | 48 | 2 | 0 | 0 | 5 | -1.2 |
| infiniteassault-123 | Cost Injection | easy | radiant | cleared | 13/24 | 88 | 88 | 2 | 0 | 0 | 20 | -2.7 |
| infiniteassault-123 | Cost Injection | hard | base | timeout | 12/24 | 35 | 35 | 3 | 0 | 1 | 14 | -0.9 |
| infiniteassault-123 | Cost Injection | hard | radiant | cleared | 14/24 | 62 | 62 | 3 | 0 | 0 | 1 | -2.3 |
| infiniteassault-124 | Firewall Breach | easy | base | cleared | 124/224 | 441 | 443 | 59 | 0 | 0 | 5 | 0.6 |
| infiniteassault-124 | Firewall Breach | easy | radiant | cleared | 11/24 | 50 | 50 | 5 | 0 | 0 | 2 | 0.1 |
| infiniteassault-124 | Firewall Breach | hard | base | cleared | 146/308 | 566 | 565 | 41 | 0 | 0 | 0 | 0.9 |
| infiniteassault-124 | Firewall Breach | hard | radiant | cleared | 11/24 | 49 | 49 | 1 | 0 | 0 | 2 | -0.5 |
| infiniteassault-125 | Zone Lock | easy | base | cleared | 164/347 | 354 | 354 | 124 | 0 | 0 | 1 | 0.5 |
| infiniteassault-125 | Zone Lock | easy | radiant | cleared | 14/24 | 44 | 44 | 11 | 0 | 0 | 0 | 1.3 |
| infiniteassault-125 | Zone Lock | hard | base | cleared | 136/296 | 338 | 338 | 84 | 0 | 0 | 1 | 0.6 |
| infiniteassault-125 | Zone Lock | hard | radiant | cleared | 8/24 | 14 | 14 | 6 | 0 | 0 | 0 | -0.4 |
| infiniteassault-127 | Bad Prompt | easy | base | cleared | 125/223 | 625 | 625 | 25 | 0 | 0 | 8 | -0.6 |
| infiniteassault-127 | Bad Prompt | easy | radiant | cleared | 12/24 | 71 | 71 | 1 | 0 | 0 | 0 | -2.1 |
| infiniteassault-127 | Bad Prompt | hard | base | cleared | 136/299 | 527 | 527 | 11 | 0 | 0 | 0 | 1.7 |
| infiniteassault-127 | Bad Prompt | hard | radiant | cleared | 11/24 | 48 | 48 | 1 | 0 | 0 | 0 | -2.1 |
| infiniteassault-129 | Signal Jammer | easy | base | cleared | 218/398 | 452 | 366 | 157 | 0 | 0 | 3 | 6.9 |
| infiniteassault-129 | Signal Jammer | easy | radiant | cleared | 14/24 | 16 | 13 | 11 | 0 | 0 | 0 | 10.7 |
| infiniteassault-129 | Signal Jammer | hard | base | cleared | 272/605 | 702 | 578 | 139 | 0 | 0 | 0 | 7.8 |
| infiniteassault-129 | Signal Jammer | hard | radiant | cleared | 9/24 | 18 | 16 | 7 | 0 | 0 | 2 | 10.1 |
| infiniteassault-130 | Prompt Injection | easy | base | cleared | 120/216 | 502 | 502 | 25 | 0 | 0 | 1 | 0.4 |
| infiniteassault-130 | Prompt Injection | easy | radiant | cleared | 13/24 | 54 | 54 | 2 | 0 | 0 | 0 | -1.1 |
| infiniteassault-130 | Prompt Injection | hard | base | cleared | 149/336 | 593 | 592 | 19 | 0 | 0 | 14 | -2.2 |
| infiniteassault-130 | Prompt Injection | hard | radiant | cleared | 12/24 | 56 | 56 | 2 | 0 | 0 | 4 | -2.1 |
| infiniteassault-132 | Ransomware | easy | base | cleared | 97/204 | 338 | 328 | 34 | 0 | 0 | 1 | 1.3 |
| infiniteassault-132 | Ransomware | easy | radiant | cleared | 11/24 | 32 | 32 | 9 | 0 | 0 | 0 | 2.2 |
| infiniteassault-132 | Ransomware | hard | base | cleared | 131/293 | 443 | 436 | 43 | 0 | 0 | 0 | 1.1 |
| infiniteassault-132 | Ransomware | hard | radiant | cleared | 12/24 | 38 | 38 | 2 | 0 | 0 | 0 | 1.0 |
| infiniteassault-134 | Rootkit Runner | easy | base | cleared | 205/420 | 383 | 377 | 173 | 0 | 0 | 1 | 2.8 |
| infiniteassault-134 | Rootkit Runner | easy | radiant | cleared | 12/24 | 22 | 21 | 11 | 0 | 0 | 3 | 7.3 |
| infiniteassault-134 | Rootkit Runner | hard | base | cleared | 263/582 | 533 | 516 | 149 | 0 | 0 | 0 | 3.2 |
| infiniteassault-134 | Rootkit Runner | hard | radiant | cleared | 12/24 | 21 | 21 | 10 | 0 | 0 | 0 | 9.4 |
| infiniteassault-135 | Mana Leech | easy | base | cleared | 225/451 | 406 | 312 | 180 | 0 | 0 | 1 | 5.9 |
| infiniteassault-135 | Mana Leech | easy | radiant | cleared | 14/24 | 11 | 11 | 11 | 0 | 0 | 0 | 9.5 |
| infiniteassault-135 | Mana Leech | hard | base | cleared | 241/542 | 595 | 472 | 125 | 0 | 0 | 0 | 6.3 |
| infiniteassault-135 | Mana Leech | hard | radiant | cleared | 13/24 | 16 | 15 | 12 | 0 | 0 | 3 | 8.7 |
| infiniteassault-136 | DDoS | easy | base | cleared | 161/265 | 434 | 434 | 91 | 0 | 0 | 49 | 45.8 |
| infiniteassault-136 | DDoS | easy | radiant | cleared | 8/24 | 18 | 18 | 6 | 0 | 0 | 2 | 22.7 |
| infiniteassault-136 | DDoS | hard | base | cleared | 187/408 | 574 | 574 | 67 | 0 | 0 | 15 | 17.4 |
| infiniteassault-136 | DDoS | hard | radiant | cleared | 6/24 | 23 | 23 | 4 | 0 | 0 | 0 | 12.5 |
| infiniteassault-137 | Patch Tuesday Exploit | easy | base | cleared | 127/222 | 571 | 571 | 39 | 0 | 0 | 19 | -2.4 |
| infiniteassault-137 | Patch Tuesday Exploit | easy | radiant | cleared | 15/24 | 74 | 74 | 2 | 0 | 0 | 0 | -1.8 |
| infiniteassault-137 | Patch Tuesday Exploit | hard | base | cleared | 125/276 | 483 | 483 | 19 | 0 | 0 | 1 | -2.0 |
| infiniteassault-137 | Patch Tuesday Exploit | hard | radiant | cleared | 10/24 | 44 | 44 | 1 | 0 | 0 | 0 | n/a |
| infiniteassault-138 | Social Engineer | easy | base | cleared | 256/489 | 553 | 460 | 188 | 0 | 0 | 0 | 22.2 |
| infiniteassault-138 | Social Engineer | easy | radiant | cleared | 14/24 | 38 | 33 | 9 | 0 | 0 | 0 | 17.6 |
| infiniteassault-138 | Social Engineer | hard | base | cleared | 312/660 | 740 | 563 | 156 | 0 | 0 | 0 | 19.5 |
| infiniteassault-138 | Social Engineer | hard | radiant | cleared | 13/24 | 23 | 19 | 6 | 0 | 0 | 0 | 17.7 |
| infiniteassault-140 | Kill Switch | easy | base | cleared | 135/263 | 533 | 502 | 50 | 0 | 0 | 2 | 2.1 |
| infiniteassault-140 | Kill Switch | easy | radiant | cleared | 16/24 | 56 | 55 | 6 | 0 | 0 | 3 | -0.1 |
| infiniteassault-140 | Kill Switch | hard | base | cleared | 191/388 | 598 | 583 | 43 | 0 | 0 | 4 | 1.8 |
| infiniteassault-140 | Kill Switch | hard | radiant | cleared | 13/24 | 38 | 34 | 4 | 0 | 0 | 0 | 1.1 |
| infiniteassault-141 | Logic Bomb | easy | base | cleared | 229/423 | 514 | 446 | 156 | 0 | 0 | 0 | 4.7 |
| infiniteassault-141 | Logic Bomb | easy | radiant | cleared | 16/24 | 22 | 21 | 11 | 0 | 0 | 2 | 6.9 |
| infiniteassault-141 | Logic Bomb | hard | base | cleared | 262/568 | 724 | 564 | 114 | 0 | 0 | 2 | 4.4 |
| infiniteassault-141 | Logic Bomb | hard | radiant | cleared | 11/24 | 20 | 20 | 10 | 0 | 0 | 2 | 6.9 |
| infiniteassault-143 | Zero-Click | easy | base | cleared | 104/203 | 165 | 165 | 94 | 0 | 0 | 22 | 20.8 |
| infiniteassault-143 | Zero-Click | easy | radiant | cleared | 12/24 | 20 | 20 | 12 | 0 | 0 | 8 | 29.5 |
| infiniteassault-143 | Zero-Click | hard | base | cleared | 153/327 | 336 | 336 | 122 | 0 | 0 | 0 | 18.7 |
| infiniteassault-143 | Zero-Click | hard | radiant | cleared | 10/24 | 23 | 23 | 5 | 0 | 0 | 0 | 9.9 |
| infiniteassault-144 | Total Compromise | easy | base | cleared | 122/214 | 180 | 180 | 89 | 0 | 0 | 0 | 46.0 |
| infiniteassault-144 | Total Compromise | easy | radiant | cleared | 13/24 | 13 | 13 | 8 | 0 | 0 | 0 | 61.0 |
| infiniteassault-144 | Total Compromise | hard | base | cleared | 165/383 | 297 | 297 | 97 | 0 | 0 | 0 | 33.3 |
| infiniteassault-144 | Total Compromise | hard | radiant | cleared | 14/24 | 23 | 23 | 9 | 0 | 0 | 0 | 44.3 |
| infiniteassault-146 | Shield Drone | easy | base | cleared | 248/513 | 510 | 397 | 201 | 0 | 0 | 5 | 4.1 |
| infiniteassault-146 | Shield Drone | easy | radiant | cleared | 7/24 | 8 | 8 | 7 | 0 | 0 | 1 | 9.8 |
| infiniteassault-146 | Shield Drone | hard | base | cleared | 262/561 | 527 | 424 | 191 | 0 | 0 | 0 | 4.4 |
| infiniteassault-146 | Shield Drone | hard | radiant | cleared | 11/24 | 26 | 15 | 8 | 0 | 0 | 0 | 27.3 |
| infiniteassault-148 | Quarantine | easy | base | cleared | 137/267 | 392 | 384 | 106 | 0 | 0 | 5 | 0.8 |
| infiniteassault-148 | Quarantine | easy | radiant | cleared | 11/24 | 29 | 29 | 8 | 0 | 0 | 0 | 0.5 |
| infiniteassault-148 | Quarantine | hard | base | cleared | 124/313 | 318 | 307 | 73 | 0 | 0 | 0 | 1.2 |
| infiniteassault-148 | Quarantine | hard | radiant | cleared | 11/24 | 23 | 23 | 10 | 0 | 0 | 0 | 1.1 |
| infiniteassault-149 | Encrypt | easy | base | timeout | 14/24 | 30 | 30 | 13 | 0 | 0 | 4 | 4.4 |
| infiniteassault-149 | Encrypt | easy | radiant | cleared | 16/24 | 26 | 26 | 15 | 0 | 0 | 8 | 7.4 |
| infiniteassault-149 | Encrypt | hard | base | timeout | 13/24 | 21 | 21 | 13 | 0 | 0 | 3 | 6.5 |
| infiniteassault-149 | Encrypt | hard | radiant | cleared | 10/24 | 10 | 10 | 9 | 0 | 0 | 0 | 7.3 |
| infiniteassault-150 | Wall of Static | easy | base | cleared | 213/424 | 431 | 357 | 159 | 0 | 0 | 3 | 20.4 |
| infiniteassault-150 | Wall of Static | easy | radiant | cleared | 11/24 | 22 | 15 | 11 | 0 | 0 | 0 | 30.3 |
| infiniteassault-150 | Wall of Static | hard | base | cleared | 320/664 | 777 | 571 | 155 | 0 | 0 | 0 | 17.9 |
| infiniteassault-150 | Wall of Static | hard | radiant | cleared | 8/24 | 11 | 9 | 4 | 0 | 0 | 1 | 14.6 |
| infiniteassault-154 | Honeypot Protocol | easy | base | cleared | 108/201 | 264 | 251 | 70 | 0 | 0 | 3 | 1.3 |
| infiniteassault-154 | Honeypot Protocol | easy | radiant | cleared | 15/24 | 41 | 40 | 10 | 0 | 0 | 0 | 1.0 |
| infiniteassault-154 | Honeypot Protocol | hard | base | cleared | 123/304 | 416 | 389 | 42 | 0 | 0 | 0 | 1.4 |
| infiniteassault-154 | Honeypot Protocol | hard | radiant | cleared | 12/24 | 33 | 31 | 3 | 0 | 0 | 0 | 7.8 |
| infiniteassault-157 | Containment Field | easy | base | cleared | 113/210 | 383 | 383 | 75 | 0 | 0 | 1 | 11.8 |
| infiniteassault-157 | Containment Field | easy | radiant | cleared | 13/24 | 41 | 41 | 12 | 0 | 0 | 0 | 18.9 |
| infiniteassault-157 | Containment Field | hard | base | cleared | 157/309 | 495 | 495 | 83 | 0 | 0 | 4 | 13.9 |
| infiniteassault-157 | Containment Field | hard | radiant | cleared | 11/24 | 41 | 41 | 5 | 0 | 0 | 0 | 5.4 |
| infiniteassault-158 | Access Denied | easy | base | cleared | 167/304 | 432 | 414 | 125 | 0 | 0 | 0 | 0.8 |
| infiniteassault-158 | Access Denied | easy | radiant | cleared | 12/24 | 42 | 41 | 5 | 0 | 0 | 6 | -9.2 |
| infiniteassault-158 | Access Denied | hard | base | cleared | 142/304 | 380 | 371 | 79 | 0 | 0 | 0 | 1.2 |
| infiniteassault-158 | Access Denied | hard | radiant | cleared | 13/24 | 28 | 24 | 7 | 0 | 0 | 1 | 0.5 |
| infiniteassault-160 | Zero Trust | easy | base | cleared | 117/236 | 357 | 353 | 64 | 0 | 0 | 0 | 7.6 |
| infiniteassault-160 | Zero Trust | easy | radiant | cleared | 14/24 | 30 | 30 | 9 | 0 | 0 | 2 | 23.7 |
| infiniteassault-160 | Zero Trust | hard | base | cleared | 192/376 | 610 | 582 | 72 | 0 | 0 | 0 | 4.7 |
| infiniteassault-160 | Zero Trust | hard | radiant | cleared | 10/24 | 16 | 15 | 6 | 0 | 0 | 3 | 5.0 |
| infiniteassault-161 | Defense in Depth | easy | base | timeout | 120/211 | 278 | 278 | 104 | 0 | 0 | 1 | 11.2 |
| infiniteassault-161 | Defense in Depth | easy | radiant | cleared | 12/24 | 14 | 14 | 10 | 0 | 0 | 2 | 10.4 |
| infiniteassault-161 | Defense in Depth | hard | base | timeout | 127/295 | 337 | 337 | 86 | 0 | 1 | 16 | 7.4 |
| infiniteassault-161 | Defense in Depth | hard | radiant | cleared | 8/24 | 14 | 14 | 7 | 0 | 0 | 2 | 10.5 |
| infiniteassault-164 | Blackhole Route | easy | base | cleared | 120/241 | 339 | 333 | 72 | 0 | 0 | 0 | 1.3 |
| infiniteassault-164 | Blackhole Route | easy | radiant | cleared | 10/24 | 23 | 22 | 5 | 0 | 0 | 0 | 0.4 |
| infiniteassault-164 | Blackhole Route | hard | base | cleared | 157/314 | 535 | 520 | 58 | 0 | 0 | 0 | 1.3 |
| infiniteassault-164 | Blackhole Route | hard | radiant | cleared | 7/24 | 22 | 22 | 2 | 0 | 0 | 1 | 1.0 |
| infiniteassault-165 | Iron Curtain | easy | base | cleared | 133/243 | 440 | 410 | 60 | 0 | 0 | 2 | 1.6 |
| infiniteassault-165 | Iron Curtain | easy | radiant | cleared | 11/24 | 24 | 24 | 4 | 0 | 0 | 1 | 1.6 |
| infiniteassault-165 | Iron Curtain | hard | base | cleared | 204/416 | 710 | 671 | 59 | 0 | 0 | 0 | 1.7 |
| infiniteassault-165 | Iron Curtain | hard | radiant | cleared | 13/24 | 39 | 39 | 4 | 0 | 0 | 0 | 1.5 |
| infiniteassault-166 | Aegis Prime | easy | base | cleared | 186/348 | 209 | 169 | 133 | 0 | 0 | 0 | 33.9 |
| infiniteassault-166 | Aegis Prime | easy | radiant | cleared | 9/24 | 7 | 5 | 5 | 0 | 0 | 18 | 33.9 |
| infiniteassault-166 | Aegis Prime | hard | base | cleared | 287/631 | 467 | 358 | 199 | 0 | 0 | 4 | 27.2 |
| infiniteassault-166 | Aegis Prime | hard | radiant | cleared | 7/24 | 9 | 9 | 6 | 0 | 0 | 0 | 35.4 |
| infiniteassault-167 | Fortress Protocol | easy | base | cleared | 97/171 | 228 | 228 | 43 | 0 | 0 | 0 | 24.6 |
| infiniteassault-167 | Fortress Protocol | easy | radiant | cleared | 11/24 | 11 | 11 | 6 | 0 | 0 | 2 | 47.7 |
| infiniteassault-167 | Fortress Protocol | hard | base | cleared | 183/368 | 451 | 451 | 72 | 0 | 0 | 5 | 18.2 |
| infiniteassault-167 | Fortress Protocol | hard | radiant | cleared | 13/24 | 22 | 22 | 7 | 0 | 0 | 6 | 36.1 |
| infiniteassault-168 | Impenetrable | easy | base | cleared | 88/182 | 197 | 197 | 49 | 0 | 0 | 0 | 31.9 |
| infiniteassault-168 | Impenetrable | easy | radiant | cleared | 15/24 | 21 | 21 | 9 | 0 | 0 | 2 | 58.2 |
| infiniteassault-168 | Impenetrable | hard | base | cleared | 167/356 | 487 | 487 | 40 | 0 | 0 | 0 | 13.7 |
| infiniteassault-168 | Impenetrable | hard | radiant | cleared | 12/24 | 31 | 31 | 3 | 0 | 0 | 0 | 106.5 |
| infiniteassault-169 | Spider Bot | easy | base | cleared | 247/478 | 424 | 352 | 196 | 0 | 0 | 0 | 5.9 |
| infiniteassault-169 | Spider Bot | easy | radiant | cleared | 17/24 | 26 | 22 | 16 | 0 | 0 | 10 | 8.6 |
| infiniteassault-169 | Spider Bot | hard | base | cleared | 259/573 | 423 | 316 | 180 | 0 | 0 | 0 | 6.0 |
| infiniteassault-169 | Spider Bot | hard | radiant | cleared | 12/24 | 18 | 12 | 9 | 0 | 0 | 1 | 8.8 |
| infiniteassault-171 | Mimic Unit | easy | base | cleared | 216/410 | 451 | 376 | 154 | 0 | 0 | 0 | 7.1 |
| infiniteassault-171 | Mimic Unit | easy | radiant | cleared | 11/24 | 18 | 15 | 10 | 0 | 0 | 2 | 11.0 |
| infiniteassault-171 | Mimic Unit | hard | base | cleared | 239/556 | 638 | 528 | 128 | 0 | 0 | 0 | 9.4 |
| infiniteassault-171 | Mimic Unit | hard | radiant | cleared | 12/24 | 31 | 21 | 9 | 0 | 0 | 1 | 11.9 |
| infiniteassault-172 | Training Data | easy | base | cleared | 102/187 | 316 | 316 | 68 | 0 | 0 | 0 | 1.0 |
| infiniteassault-172 | Training Data | easy | radiant | cleared | 15/24 | 31 | 31 | 9 | 0 | 0 | 7 | 2.0 |
| infiniteassault-172 | Training Data | hard | base | cleared | 150/308 | 545 | 545 | 59 | 0 | 0 | 0 | 1.8 |
| infiniteassault-172 | Training Data | hard | radiant | cleared | 11/24 | 36 | 35 | 5 | 0 | 0 | 0 | 1.4 |
| infiniteassault-174 | Scrape the Graveyard | easy | base | cleared | 122/241 | 431 | 431 | 68 | 0 | 0 | 0 | 1.1 |
| infiniteassault-174 | Scrape the Graveyard | easy | radiant | cleared | 13/24 | 40 | 40 | 6 | 0 | 0 | 0 | 1.1 |
| infiniteassault-174 | Scrape the Graveyard | hard | base | cleared | 162/330 | 580 | 580 | 47 | 0 | 0 | 4 | 1.2 |
| infiniteassault-174 | Scrape the Graveyard | hard | radiant | cleared | 13/24 | 44 | 44 | 5 | 0 | 0 | 0 | 1.9 |
| infiniteassault-175 | Fingerprinter | easy | base | cleared | 259/490 | 484 | 396 | 213 | 0 | 0 | 0 | 4.7 |
| infiniteassault-175 | Fingerprinter | easy | radiant | cleared | 17/24 | 28 | 19 | 10 | 0 | 0 | 4 | 5.2 |
| infiniteassault-175 | Fingerprinter | hard | base | cleared | 271/567 | 459 | 366 | 192 | 0 | 0 | 0 | 4.9 |
| infiniteassault-175 | Fingerprinter | hard | radiant | cleared | 10/24 | 21 | 12 | 7 | 0 | 0 | 3 | 5.7 |
| infiniteassault-176 | Imitation Engine | easy | base | cleared | 216/457 | 437 | 369 | 157 | 0 | 0 | 0 | 14.5 |
| infiniteassault-176 | Imitation Engine | easy | radiant | cleared | 13/24 | 36 | 32 | 9 | 0 | 0 | 2 | 16.3 |
| infiniteassault-176 | Imitation Engine | hard | base | cleared | 300/663 | 718 | 572 | 151 | 0 | 0 | 0 | 14.6 |
| infiniteassault-176 | Imitation Engine | hard | radiant | cleared | 11/24 | 23 | 18 | 5 | 0 | 0 | 3 | 17.3 |
| infiniteassault-177 | Predictive Text | easy | base | cleared | 116/211 | 380 | 380 | 66 | 0 | 0 | 0 | 2.2 |
| infiniteassault-177 | Predictive Text | easy | radiant | cleared | 11/24 | 42 | 42 | 8 | 0 | 0 | 0 | 2.9 |
| infiniteassault-177 | Predictive Text | hard | base | cleared | 137/294 | 435 | 435 | 58 | 0 | 0 | 0 | 1.7 |
| infiniteassault-177 | Predictive Text | hard | radiant | cleared | 12/24 | 36 | 35 | 7 | 0 | 0 | 7 | 2.2 |
| infiniteassault-178 | Dataset Thief | easy | base | cleared | 238/434 | 536 | 457 | 178 | 0 | 0 | 1 | 12.1 |
| infiniteassault-178 | Dataset Thief | easy | radiant | cleared | 13/24 | 14 | 12 | 12 | 0 | 0 | 2 | 18.8 |
| infiniteassault-178 | Dataset Thief | hard | base | cleared | 293/663 | 588 | 431 | 158 | 0 | 0 | 0 | 12.1 |
| infiniteassault-178 | Dataset Thief | hard | radiant | cleared | 8/24 | 10 | 10 | 6 | 0 | 0 | 0 | 19.2 |
| infiniteassault-179 | Style Transfer | easy | base | cleared | 111/201 | 342 | 342 | 78 | 0 | 0 | 2 | 14.4 |
| infiniteassault-179 | Style Transfer | easy | radiant | cleared | 12/24 | 21 | 21 | 11 | 0 | 0 | 4 | 19.3 |
| infiniteassault-179 | Style Transfer | hard | base | cleared | 125/281 | 372 | 372 | 67 | 0 | 0 | 0 | 10.1 |
| infiniteassault-179 | Style Transfer | hard | radiant | cleared | 10/24 | 22 | 22 | 7 | 0 | 0 | 0 | 21.4 |
| infiniteassault-182 | Model Theft | easy | base | cleared | 103/210 | 173 | 173 | 83 | 0 | 0 | 0 | 18.9 |
| infiniteassault-182 | Model Theft | easy | radiant | cleared | 16/24 | 21 | 21 | 15 | 0 | 0 | 3 | 24.2 |
| infiniteassault-182 | Model Theft | hard | base | cleared | 159/366 | 393 | 393 | 88 | 0 | 0 | 0 | 15.7 |
| infiniteassault-182 | Model Theft | hard | radiant | cleared | 9/24 | 10 | 10 | 6 | 0 | 0 | 0 | 25.4 |
| infiniteassault-183 | Shadow Library | easy | base | cleared | 109/231 | 355 | 329 | 69 | 0 | 0 | 14 | 1.3 |
| infiniteassault-183 | Shadow Library | easy | radiant | cleared | 18/24 | 48 | 48 | 13 | 0 | 0 | 2 | 0.9 |
| infiniteassault-183 | Shadow Library | hard | base | cleared | 142/315 | 428 | 410 | 56 | 0 | 0 | 0 | 1.5 |
| infiniteassault-183 | Shadow Library | hard | radiant | cleared | 15/24 | 41 | 38 | 7 | 0 | 0 | 0 | 0.5 |
| infiniteassault-184 | Pattern Recognition | easy | base | cleared | 153/268 | 410 | 410 | 109 | 0 | 0 | 0 | -1.2 |
| infiniteassault-184 | Pattern Recognition | easy | radiant | cleared | 12/24 | 29 | 29 | 6 | 0 | 0 | 0 | -1.8 |
| infiniteassault-184 | Pattern Recognition | hard | base | cleared | 125/286 | 381 | 381 | 70 | 0 | 0 | 0 | -1.3 |
| infiniteassault-184 | Pattern Recognition | hard | radiant | cleared | 16/24 | 58 | 58 | 12 | 0 | 0 | 0 | -1.5 |
| infiniteassault-185 | Deep Crawler | easy | base | cleared | 245/450 | 527 | 437 | 171 | 0 | 0 | 2 | 13.8 |
| infiniteassault-185 | Deep Crawler | easy | radiant | cleared | 16/24 | 22 | 19 | 15 | 0 | 0 | 7 | 24.3 |
| infiniteassault-185 | Deep Crawler | hard | base | cleared | 320/673 | 742 | 588 | 178 | 0 | 0 | 0 | 14.3 |
| infiniteassault-185 | Deep Crawler | hard | radiant | cleared | 8/24 | 6 | 4 | 5 | 0 | 0 | 0 | 24.1 |
| infiniteassault-186 | Identity Theft | easy | base | cleared | 119/219 | 415 | 418 | 49 | 0 | 0 | 0 | 1.9 |
| infiniteassault-186 | Identity Theft | easy | radiant | cleared | 9/24 | 27 | 27 | 5 | 0 | 0 | 0 | 2.4 |
| infiniteassault-186 | Identity Theft | hard | base | cleared | 178/401 | 634 | 634 | 32 | 0 | 0 | 0 | 1.7 |
| infiniteassault-186 | Identity Theft | hard | radiant | cleared | 11/24 | 33 | 33 | 3 | 0 | 0 | 0 | 2.3 |
| infiniteassault-187 | Fine-Tuner | easy | base | cleared | 214/409 | 443 | 368 | 158 | 0 | 0 | 0 | 7.8 |
| infiniteassault-187 | Fine-Tuner | easy | radiant | cleared | 15/24 | 23 | 16 | 10 | 0 | 0 | 0 | 11.4 |
| infiniteassault-187 | Fine-Tuner | hard | base | cleared | 251/527 | 560 | 448 | 134 | 0 | 0 | 2 | 7.7 |
| infiniteassault-187 | Fine-Tuner | hard | radiant | cleared | 8/24 | 9 | 8 | 4 | 0 | 0 | 1 | 12.3 |
| infiniteassault-188 | Overfitting | easy | base | cleared | 122/228 | 453 | 430 | 46 | 0 | 0 | 0 | 1.9 |
| infiniteassault-188 | Overfitting | easy | radiant | cleared | 12/24 | 41 | 41 | 4 | 0 | 0 | 0 | 0.5 |
| infiniteassault-188 | Overfitting | hard | base | cleared | 205/418 | 655 | 638 | 52 | 0 | 0 | 0 | 2.1 |
| infiniteassault-188 | Overfitting | hard | radiant | cleared | 8/24 | 26 | 26 | 3 | 0 | 0 | 1 | 1.5 |
| infiniteassault-189 | Clone Vat | easy | base | cleared | 83/168 | 135 | 127 | 45 | 0 | 0 | 3 | 4.0 |
| infiniteassault-189 | Clone Vat | easy | radiant | cleared | 14/24 | 25 | 25 | 11 | 0 | 0 | 0 | 7.1 |
| infiniteassault-189 | Clone Vat | hard | base | cleared | 161/335 | 409 | 396 | 65 | 0 | 0 | 5 | 2.4 |
| infiniteassault-189 | Clone Vat | hard | radiant | cleared | 6/24 | 19 | 17 | 1 | 0 | 0 | 0 | 2.0 |
| infiniteassault-191 | Ground Truth | easy | base | cleared | 213/408 | 367 | 316 | 170 | 0 | 0 | 3 | 8.3 |
| infiniteassault-191 | Ground Truth | easy | radiant | cleared | 11/24 | 9 | 9 | 11 | 0 | 0 | 0 | 13.7 |
| infiniteassault-191 | Ground Truth | hard | base | cleared | 250/574 | 512 | 407 | 154 | 0 | 0 | 2 | 9.0 |
| infiniteassault-191 | Ground Truth | hard | radiant | cleared | 8/24 | 12 | 9 | 6 | 0 | 0 | 0 | 13.3 |
| infiniteassault-192 | Become the Data | easy | base | cleared | 92/174 | 300 | 300 | 8 | 0 | 0 | 5 | -0.0 |
| infiniteassault-192 | Become the Data | easy | radiant | cleared | 12/24 | 27 | 27 | 2 | 0 | 0 | 0 | 2.0 |
| infiniteassault-192 | Become the Data | hard | base | cleared | 165/368 | 501 | 501 | 19 | 0 | 0 | 0 | 1.2 |
| infiniteassault-192 | Become the Data | hard | radiant | cleared | 10/24 | 29 | 29 | 1 | 0 | 0 | 0 | 1.4 |
| infiniteassault-194 | Compute Shard | easy | base | cleared | 261/496 | 588 | 492 | 193 | 0 | 0 | 1 | 2.0 |
| infiniteassault-194 | Compute Shard | easy | radiant | cleared | 13/24 | 28 | 23 | 9 | 0 | 0 | 14 | 2.5 |
| infiniteassault-194 | Compute Shard | hard | base | cleared | 264/576 | 555 | 441 | 140 | 0 | 0 | 0 | 2.6 |
| infiniteassault-194 | Compute Shard | hard | radiant | cleared | 12/24 | 24 | 22 | 7 | 0 | 0 | 0 | 2.7 |
| infiniteassault-195 | Power Law | easy | base | cleared | 199/411 | 368 | 303 | 167 | 0 | 0 | 2 | 11.9 |
| infiniteassault-195 | Power Law | easy | radiant | cleared | 13/24 | 14 | 13 | 13 | 0 | 0 | 0 | 17.2 |
| infiniteassault-195 | Power Law | hard | base | cleared | 267/569 | 574 | 469 | 173 | 0 | 0 | 0 | 11.7 |
| infiniteassault-195 | Power Law | hard | radiant | cleared | 11/24 | 14 | 10 | 9 | 0 | 0 | 0 | 17.9 |
| infiniteassault-196 | Emergent Behavior | easy | base | cleared | 120/228 | 232 | 232 | 115 | 0 | 0 | 7 | 11.5 |
| infiniteassault-196 | Emergent Behavior | easy | radiant | cleared | 8/24 | 9 | 9 | 7 | 0 | 0 | 0 | 50.5 |
| infiniteassault-196 | Emergent Behavior | hard | base | cleared | 120/298 | 263 | 263 | 100 | 0 | 0 | 0 | 11.1 |
| infiniteassault-196 | Emergent Behavior | hard | radiant | cleared | 11/24 | 15 | 15 | 8 | 0 | 0 | 6 | 20.4 |
| infiniteassault-199 | Patient Machine | easy | base | cleared | 206/404 | 367 | 311 | 163 | 0 | 0 | 7 | 4.3 |
| infiniteassault-199 | Patient Machine | easy | radiant | cleared | 14/24 | 15 | 9 | 10 | 0 | 0 | 1 | 3.3 |
| infiniteassault-199 | Patient Machine | hard | base | cleared | 256/536 | 621 | 478 | 124 | 0 | 0 | 2 | 4.5 |
| infiniteassault-199 | Patient Machine | hard | radiant | cleared | 11/24 | 11 | 10 | 9 | 0 | 0 | 0 | 4.9 |
| infiniteassault-202 | Exponent | easy | base | cleared | 230/435 | 757 | 610 | 100 | 0 | 0 | 3 | 4.7 |
| infiniteassault-202 | Exponent | easy | radiant | cleared | 12/24 | 16 | 13 | 9 | 0 | 0 | 0 | 6.0 |
| infiniteassault-202 | Exponent | hard | base | cleared | 294/651 | 945 | 722 | 80 | 0 | 0 | 1 | 4.2 |
| infiniteassault-202 | Exponent | hard | radiant | cleared | 13/24 | 14 | 10 | 11 | 0 | 0 | 0 | 7.7 |
| infiniteassault-203 | Compute Cluster | easy | base | cleared | 107/210 | 216 | 206 | 75 | 0 | 0 | 0 | 2.0 |
| infiniteassault-203 | Compute Cluster | easy | radiant | cleared | 17/24 | 48 | 47 | 15 | 0 | 0 | 0 | 0.9 |
| infiniteassault-203 | Compute Cluster | hard | base | cleared | 119/285 | 303 | 296 | 80 | 0 | 0 | 0 | 1.4 |
| infiniteassault-203 | Compute Cluster | hard | radiant | cleared | 14/24 | 20 | 20 | 12 | 0 | 0 | 0 | 0.7 |
| infiniteassault-207 | Recursive Self-Improvement | easy | base | cleared | 102/200 | 266 | 267 | 96 | 0 | 0 | 5 | 7.2 |
| infiniteassault-207 | Recursive Self-Improvement | easy | radiant | cleared | 9/24 | 16 | 16 | 17 | 0 | 0 | 0 | 13.0 |
| infiniteassault-207 | Recursive Self-Improvement | hard | base | cleared | 147/300 | 411 | 411 | 115 | 0 | 0 | 1 | 5.2 |
| infiniteassault-207 | Recursive Self-Improvement | hard | radiant | cleared | 11/24 | 19 | 19 | 9 | 0 | 0 | 0 | 8.5 |
| infiniteassault-211 | Grey Goo | easy | base | cleared | 207/429 | 991 | 801 | 26 | 0 | 0 | 0 | 8.8 |
| infiniteassault-211 | Grey Goo | easy | radiant | neverPlayed | 8/24 | 38 | 28 | 3 | 0 | 0 | 0 | 1.0 |
| infiniteassault-211 | Grey Goo | hard | base | cleared | 319/681 | 1129 | 864 | 11 | 0 | 0 | 0 | 6.6 |
| infiniteassault-211 | Grey Goo | hard | radiant | neverPlayed | 11/24 | 44 | 41 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-213 | The Long Game | easy | base | cleared | 221/387 | 365 | 306 | 118 | 0 | 0 | 1 | 2.7 |
| infiniteassault-213 | The Long Game | easy | radiant | cleared | 14/24 | 23 | 18 | 7 | 0 | 0 | 0 | -3.1 |
| infiniteassault-213 | The Long Game | hard | base | cleared | 291/644 | 784 | 569 | 84 | 0 | 0 | 1 | 2.5 |
| infiniteassault-213 | The Long Game | hard | radiant | cleared | 13/24 | 35 | 22 | 1 | 0 | 0 | 0 | -2.7 |
| infiniteassault-214 | Emergent Mind | easy | base | cleared | 210/395 | 482 | 345 | 80 | 0 | 0 | 0 | 10.6 |
| infiniteassault-214 | Emergent Mind | easy | radiant | cleared | 11/24 | 17 | 13 | 8 | 0 | 0 | 0 | 13.8 |
| infiniteassault-214 | Emergent Mind | hard | base | cleared | 305/667 | 827 | 598 | 91 | 0 | 0 | 0 | 9.4 |
| infiniteassault-214 | Emergent Mind | hard | radiant | cleared | 15/24 | 33 | 26 | 8 | 0 | 0 | 0 | 13.0 |
| infiniteassault-218 | Thousand Faces | easy | base | cleared | 140/257 | 413 | 395 | 73 | 0 | 0 | 0 | 1.2 |
| infiniteassault-218 | Thousand Faces | easy | radiant | cleared | 9/24 | 40 | 40 | 4 | 0 | 0 | 0 | 1.0 |
| infiniteassault-218 | Thousand Faces | hard | base | cleared | 132/353 | 459 | 443 | 56 | 0 | 0 | 0 | 1.2 |
| infiniteassault-218 | Thousand Faces | hard | radiant | cleared | 6/24 | 12 | 12 | 2 | 0 | 0 | 2 | 1.0 |
| infiniteassault-220 | Last Lap | easy | base | cleared | 143/263 | 381 | 381 | 121 | 0 | 0 | 0 | 10.6 |
| infiniteassault-220 | Last Lap | easy | radiant | cleared | 15/24 | 33 | 33 | 14 | 0 | 0 | 7 | 25.1 |
| infiniteassault-220 | Last Lap | hard | base | cleared | 158/365 | 336 | 336 | 129 | 0 | 0 | 0 | 13.0 |
| infiniteassault-220 | Last Lap | hard | radiant | cleared | 5/24 | 12 | 12 | 5 | 0 | 0 | 0 | 19.9 |
| infiniteassault-223 | Loopback, Who Returns | easy | base | cleared | 206/384 | 305 | 241 | 119 | 0 | 0 | 0 | 15.9 |
| infiniteassault-223 | Loopback, Who Returns | easy | radiant | cleared | 13/24 | 15 | 13 | 10 | 0 | 0 | 0 | 24.5 |
| infiniteassault-223 | Loopback, Who Returns | hard | base | cleared | 281/634 | 606 | 460 | 131 | 0 | 0 | 4 | 16.7 |
| infiniteassault-223 | Loopback, Who Returns | hard | radiant | cleared | 11/24 | 18 | 13 | 7 | 0 | 0 | 0 | 27.3 |
| infiniteassault-224 | Rollback Point | easy | base | cleared | 126/265 | 338 | 338 | 79 | 0 | 0 | 0 | 19.8 |
| infiniteassault-224 | Rollback Point | easy | radiant | cleared | 13/24 | 28 | 28 | 8 | 0 | 0 | 0 | 45.3 |
| infiniteassault-224 | Rollback Point | hard | base | cleared | 181/420 | 525 | 525 | 71 | 0 | 0 | 0 | 22.0 |
| infiniteassault-224 | Rollback Point | hard | radiant | cleared | 14/24 | 23 | 23 | 7 | 0 | 0 | 25 | 21.0 |
| infiniteassault-226 | Escalated Privileges | easy | base | cleared | 151/286 | 464 | 464 | 82 | 0 | 0 | 2 | 4.2 |
| infiniteassault-226 | Escalated Privileges | easy | radiant | cleared | 10/24 | 26 | 26 | 4 | 0 | 0 | 4 | 8.0 |
| infiniteassault-226 | Escalated Privileges | hard | base | cleared | 202/436 | 606 | 606 | 81 | 0 | 0 | 0 | 3.3 |
| infiniteassault-226 | Escalated Privileges | hard | radiant | cleared | 15/24 | 59 | 59 | 8 | 0 | 0 | 0 | 6.0 |
| infiniteassault-228 | Firewall Rules | easy | base | cleared | 124/271 | 483 | 451 | 48 | 0 | 0 | 0 | 1.6 |
| infiniteassault-228 | Firewall Rules | easy | radiant | cleared | 13/24 | 37 | 37 | 6 | 0 | 0 | 3 | 1.5 |
| infiniteassault-228 | Firewall Rules | hard | base | cleared | 165/390 | 552 | 535 | 55 | 0 | 0 | 5 | 2.0 |
| infiniteassault-228 | Firewall Rules | hard | radiant | cleared | 11/24 | 32 | 31 | 2 | 0 | 0 | 0 | 1.0 |
| infiniteassault-234 | Degenerate Output | easy | base | cleared | 139/273 | 311 | 311 | 101 | 0 | 0 | 11 | 17.0 |
| infiniteassault-234 | Degenerate Output | easy | radiant | cleared | 10/24 | 21 | 21 | 8 | 0 | 0 | 0 | 38.9 |
| infiniteassault-234 | Degenerate Output | hard | base | cleared | 143/333 | 407 | 407 | 73 | 0 | 0 | 0 | 12.7 |
| infiniteassault-234 | Degenerate Output | hard | radiant | cleared | 16/24 | 48 | 48 | 9 | 0 | 0 | 0 | 13.3 |
| infiniteassault-236 | Infinite Assault | easy | base | cleared | 98/185 | 116 | 116 | 67 | 0 | 0 | 3 | 44.4 |
| infiniteassault-236 | Infinite Assault | easy | radiant | cleared | 12/24 | 13 | 13 | 10 | 0 | 0 | 0 | 48.5 |
| infiniteassault-236 | Infinite Assault | hard | base | cleared | 166/336 | 241 | 241 | 117 | 0 | 0 | 2 | 36.0 |
| infiniteassault-236 | Infinite Assault | hard | radiant | cleared | 9/24 | 11 | 11 | 7 | 0 | 0 | 1 | 65.8 |
| infiniteassault-237 | Thermal Runaway | easy | base | cleared | 198/381 | 191 | 168 | 138 | 0 | 0 | 0 | 25.4 |
| infiniteassault-237 | Thermal Runaway | easy | radiant | cleared | 12/24 | 14 | 10 | 9 | 0 | 0 | 0 | 39.6 |
| infiniteassault-237 | Thermal Runaway | hard | base | cleared | 308/702 | 364 | 280 | 248 | 0 | 0 | 0 | 26.9 |
| infiniteassault-237 | Thermal Runaway | hard | radiant | cleared | 10/24 | 12 | 10 | 10 | 0 | 0 | 0 | 44.0 |
| infiniteassault-238 | Critical Temperature | easy | base | cleared | 156/288 | 463 | 448 | 83 | 0 | 0 | 0 | 2.0 |
| infiniteassault-238 | Critical Temperature | easy | radiant | cleared | 15/24 | 42 | 41 | 9 | 0 | 0 | 0 | 1.9 |
| infiniteassault-238 | Critical Temperature | hard | base | cleared | 205/453 | 627 | 602 | 65 | 0 | 0 | 0 | 1.8 |
| infiniteassault-238 | Critical Temperature | hard | radiant | cleared | 6/24 | 15 | 15 | 3 | 0 | 0 | 0 | 1.5 |
| infiniteassault-240 | Unpatched | easy | base | cleared | 143/272 | 500 | 480 | 62 | 0 | 0 | 2 | 2.4 |
| infiniteassault-240 | Unpatched | easy | radiant | cleared | 13/24 | 52 | 48 | 1 | 0 | 0 | 0 | 1.5 |
| infiniteassault-240 | Unpatched | hard | base | cleared | 180/424 | 604 | 579 | 39 | 0 | 0 | 0 | 2.1 |
| infiniteassault-240 | Unpatched | hard | radiant | cleared | 14/24 | 80 | 66 | 2 | 0 | 0 | 0 | 1.5 |
| meditative-011 | Double Header | easy | base | cleared | 92/159 | 164 | 158 | 13 | 0 | 0 | 0 | 1.9 |
| meditative-011 | Double Header | hard | base | cleared | 141/302 | 257 | 250 | 15 | 0 | 0 | 0 | 2.5 |
| meditative-022 | Mind Games | easy | base | cleared | 83/181 | 279 | 280 | 43 | 0 | 0 | 0 | -3.7 |
| meditative-022 | Mind Games | hard | base | cleared | 109/249 | 357 | 357 | 28 | 0 | 0 | 0 | -3.7 |
| meditative-026 | Alternate Fate | easy | base | cleared | 68/142 | 170 | 164 | 7 | 0 | 0 | 0 | 2.3 |
| meditative-026 | Alternate Fate | hard | base | cleared | 138/279 | 430 | 410 | 9 | 0 | 0 | 0 | 2.5 |
| meditative-053 | Prestige | easy | base | cleared | 125/210 | 181 | 46 | 4 | 0 | 0 | 0 | 40.8 |
| meditative-053 | Prestige | hard | base | cleared | 106/250 | 133 | 24 | 6 | 0 | 0 | 0 | 5.7 |
| meditative-055 | Dragon Fruit | easy | base | cleared | 123/234 | 442 | 442 | 58 | 0 | 0 | 0 | 0.2 |
| meditative-055 | Dragon Fruit | hard | base | cleared | 151/346 | 576 | 576 | 42 | 0 | 0 | 0 | 0.9 |
| meditative-060 | Eschews | easy | base | cleared | 69/158 | 159 | 148 | 47 | 0 | 0 | 0 | 3.4 |
| meditative-060 | Eschews | hard | base | cleared | 129/305 | 304 | 295 | 57 | 0 | 0 | 0 | 3.2 |
| meditative-069 | The Maestro | easy | base | cleared | 186/362 | 402 | 330 | 64 | 0 | 0 | 0 | 10.1 |
| meditative-069 | The Maestro | hard | base | cleared | 306/656 | 900 | 626 | 61 | 0 | 0 | 0 | 9.5 |
| meditative-075 | Ever Growing Tree | easy | base | cleared | 84/130 | 250 | 220 | 18 | 0 | 0 | 0 | 2.5 |
| meditative-075 | Ever Growing Tree | hard | base | cleared | 121/290 | 338 | 325 | 33 | 0 | 0 | 0 | 2.6 |
| meditative-098 | Showdown | easy | base | cleared | 114/229 | 168 | 129 | 1 | 0 | 0 | 0 | -1.7 |
| meditative-098 | Showdown | hard | base | cleared | 98/216 | 123 | 90 | 1 | 0 | 0 | 0 | -1.3 |
| meditative-099 | Paranoia | easy | base | cleared | 87/166 | 260 | 256 | 65 | 0 | 0 | 0 | 3.3 |
| meditative-099 | Paranoia | hard | base | cleared | 113/253 | 314 | 311 | 51 | 0 | 0 | 0 | 3.2 |
| meditativeplus-002 | NA Tech | easy | base | cleared | 94/184 | 95 | 95 | 91 | 0 | 0 | 0 | 0.8 |
| meditativeplus-002 | NA Tech | hard | base | cleared | 122/240 | 121 | 121 | 106 | 0 | 0 | 0 | 0.8 |
| meditativeplus-003 | Wondrous Wand | easy | base | cleared | 110/209 | 239 | 239 | 89 | 0 | 0 | 0 | 0.6 |
| meditativeplus-003 | Wondrous Wand | hard | base | cleared | 113/233 | 241 | 241 | 75 | 0 | 0 | 0 | 0.2 |
| meditativeplus-004 | Jlockeed's Supply Drop | easy | base | cleared | 124/223 | 323 | 323 | 77 | 0 | 0 | 0 | -1.7 |
| meditativeplus-004 | Jlockeed's Supply Drop | hard | base | cleared | 149/300 | 416 | 416 | 55 | 0 | 0 | 0 | -0.9 |
| meditativeplus-005 | Spirit Lash | easy | base | cleared | 109/231 | 246 | 246 | 88 | 0 | 0 | 0 | 6.7 |
| meditativeplus-005 | Spirit Lash | hard | base | cleared | 113/235 | 248 | 248 | 66 | 0 | 0 | 0 | 4.8 |
| meditativeplus-013 | Living Health Potion | easy | base | cleared | 250/447 | 378 | 314 | 211 | 0 | 0 | 0 | 6.4 |
| meditativeplus-013 | Living Health Potion | hard | base | cleared | 211/492 | 342 | 258 | 144 | 0 | 0 | 0 | 6.8 |
| meditativeplus-014 | Potion Pouch | easy | base | cleared | 130/228 | 223 | 223 | 106 | 0 | 0 | 0 | 2.0 |
| meditativeplus-014 | Potion Pouch | hard | base | cleared | 119/241 | 236 | 236 | 84 | 0 | 0 | 0 | 1.2 |
| meditativeplus-020 | KY's Competition | easy | base | cleared | 120/211 | 131 | 131 | 113 | 0 | 0 | 0 | -1.6 |
| meditativeplus-020 | KY's Competition | hard | base | cleared | 178/337 | 216 | 216 | 152 | 0 | 0 | 0 | -1.6 |
| meditativeplus-022 | Mountaintop Garden | easy | base | cleared | 91/172 | 204 | 200 | 61 | 0 | 0 | 0 | 2.6 |
| meditativeplus-022 | Mountaintop Garden | hard | base | cleared | 98/223 | 301 | 295 | 49 | 0 | 0 | 0 | 3.3 |
| meditativeplus-023 | Enormous Elderberry | easy | base | cleared | 119/203 | 370 | 370 | 59 | 0 | 0 | 0 | 1.1 |
| meditativeplus-023 | Enormous Elderberry | hard | base | cleared | 174/344 | 609 | 609 | 45 | 0 | 0 | 0 | 0.8 |
| stinkyscape-002 | Absolute Zero | easy | base | cleared | 68/142 | 189 | 190 | 40 | 0 | 0 | 0 | 26.4 |
| stinkyscape-002 | Absolute Zero | hard | base | cleared | 100/231 | 333 | 333 | 58 | 0 | 0 | 0 | 11.4 |
| stinkyscape-004 | Olivian Expanse | easy | base | cleared | 67/142 | 215 | 216 | 10 | 0 | 0 | 0 | -2.1 |
| stinkyscape-004 | Olivian Expanse | hard | base | cleared | 134/280 | 424 | 427 | 14 | 0 | 0 | 0 | -1.9 |
| stinkyscape-007 | The Greatest Heist | easy | base | cleared | 90/181 | 233 | 231 | 63 | 0 | 0 | 0 | 1.5 |
| stinkyscape-007 | The Greatest Heist | hard | base | cleared | 129/265 | 417 | 407 | 69 | 0 | 0 | 0 | 1.5 |
| stinkyscape-008 | Conjure Rosemary Triscuit | easy | base | cleared | 81/146 | 231 | 231 | 48 | 0 | 0 | 0 | 0.5 |
| stinkyscape-008 | Conjure Rosemary Triscuit | hard | base | cleared | 118/244 | 321 | 321 | 56 | 0 | 0 | 0 | 1.3 |
| stinkyscape-009 | Ticketed | easy | base | cleared | 81/154 | 131 | 131 | 70 | 0 | 0 | 0 | 21.1 |
| stinkyscape-009 | Ticketed | hard | base | cleared | 124/268 | 286 | 286 | 85 | 0 | 0 | 0 | 17.3 |
| stinkyscape-010 | Otherworldly Endurance | easy | base | cleared | 98/180 | 221 | 221 | 68 | 0 | 0 | 0 | 2.3 |
| stinkyscape-010 | Otherworldly Endurance | hard | base | cleared | 108/245 | 313 | 313 | 65 | 0 | 0 | 0 | 2.1 |
| stinkyscape-011 | Otherworldly Magic | easy | base | cleared | 78/172 | 170 | 170 | 59 | 0 | 0 | 0 | 2.2 |
| stinkyscape-011 | Otherworldly Magic | hard | base | cleared | 108/229 | 255 | 256 | 61 | 0 | 0 | 0 | 1.7 |
| stinkyscape-017 | Imprisoned Ship Smith | easy | base | cleared | 230/461 | 404 | 361 | 184 | 0 | 0 | 0 | -0.4 |
| stinkyscape-017 | Imprisoned Ship Smith | hard | base | cleared | 246/523 | 508 | 399 | 146 | 0 | 0 | 0 | 0.1 |
| stinkyscape-019 | Imprisoned Cosmality | easy | base | cleared | 181/365 | 252 | 205 | 155 | 0 | 0 | 0 | 3.7 |
| stinkyscape-019 | Imprisoned Cosmality | hard | base | cleared | 239/511 | 350 | 301 | 183 | 0 | 0 | 0 | 2.9 |
| stinkyscape-021 | Moonberry | easy | base | cleared | 142/288 | 318 | 318 | 109 | 0 | 0 | 0 | 0.3 |
| stinkyscape-021 | Moonberry | hard | base | cleared | 143/300 | 325 | 325 | 88 | 0 | 0 | 0 | 1.1 |
| stinkyscape-022 | Gatinsir | easy | base | cleared | 67/149 | 155 | 153 | 14 | 0 | 0 | 0 | 2.3 |
| stinkyscape-022 | Gatinsir | hard | base | cleared | 137/296 | 430 | 413 | 42 | 0 | 0 | 0 | 2.6 |
| stinkyscape-024 | Warp Disruptor II | easy | base | cleared | 92/177 | 202 | 202 | 80 | 0 | 0 | 0 | 2.7 |
| stinkyscape-024 | Warp Disruptor II | hard | base | cleared | 104/233 | 212 | 212 | 89 | 0 | 0 | 0 | 9.2 |
| stinkyscape-032 | Drone Core VI | easy | base | cleared | 116/225 | 97 | 97 | 73 | 0 | 0 | 0 | 34.4 |
| stinkyscape-032 | Drone Core VI | hard | base | cleared | 161/326 | 183 | 183 | 131 | 0 | 0 | 0 | 37.5 |
| stinkyscape-036 | Drone Core X | easy | base | cleared | 45/79 | 16 | 16 | 8 | 0 | 0 | 0 | 31.7 |
| stinkyscape-036 | Drone Core X | hard | base | cleared | 146/353 | 83 | 83 | 17 | 0 | 0 | 0 | 31.7 |
| stinkyscape-039 | The Narcor Project | easy | base | cleared | 96/202 | 353 | 342 | 32 | 0 | 0 | 0 | 0.5 |
| stinkyscape-039 | The Narcor Project | hard | base | cleared | 135/317 | 434 | 410 | 35 | 0 | 0 | 0 | 0.2 |
| stinkyscape-040 | Poison Capsule | easy | base | cleared | 229/442 | 599 | 482 | 130 | 0 | 0 | 0 | 3.5 |
| stinkyscape-040 | Poison Capsule | hard | base | cleared | 304/674 | 877 | 665 | 84 | 0 | 0 | 0 | 4.0 |
| stinkyscape-046 | Circadian Halfcycler | easy | base | cleared | 100/202 | 112 | 111 | 92 | 0 | 0 | 0 | 2.5 |
| stinkyscape-046 | Circadian Halfcycler | hard | base | cleared | 121/244 | 137 | 131 | 104 | 0 | 0 | 0 | 1.9 |
| stinkyscape-047 | Circadian Fullcycler | easy | base | cleared | 58/127 | 52 | 50 | 40 | 0 | 0 | 0 | 4.0 |
| stinkyscape-047 | Circadian Fullcycler | hard | base | cleared | 132/284 | 183 | 175 | 92 | 0 | 0 | 0 | 2.8 |
| stinkyscape-049 | Spacemine | easy | base | cleared | 173/340 | 220 | 193 | 154 | 0 | 0 | 0 | 7.8 |
| stinkyscape-049 | Spacemine | hard | base | cleared | 212/480 | 313 | 248 | 158 | 0 | 0 | 0 | 8.3 |
| stinkyscape-052 | Cannon v3 | easy | base | cleared | 242/429 | 458 | 386 | 173 | 0 | 0 | 0 | 0.5 |
| stinkyscape-052 | Cannon v3 | hard | base | cleared | 230/499 | 676 | 513 | 98 | 0 | 0 | 0 | 1.8 |
| stinkyscape-055 | Repeater v4 | easy | base | cleared | 219/425 | 623 | 519 | 129 | 0 | 0 | 0 | 1.4 |
| stinkyscape-055 | Repeater v4 | hard | base | cleared | 296/631 | 881 | 667 | 81 | 0 | 0 | 0 | 1.3 |
| stinkyscape-058 | Beam v5 | easy | base | cleared | 167/323 | 231 | 191 | 87 | 0 | 0 | 0 | 2.0 |
| stinkyscape-058 | Beam v5 | hard | base | cleared | 275/609 | 611 | 443 | 129 | 0 | 0 | 0 | 1.9 |
| stinkyscape-059 | Repeater v5 | easy | base | cleared | 186/343 | 394 | 306 | 87 | 0 | 0 | 0 | 4.4 |
| stinkyscape-059 | Repeater v5 | hard | base | cleared | 281/591 | 861 | 564 | 43 | 0 | 0 | 0 | 1.8 |
| stinkyscape-061 | Flak v5 | easy | base | cleared | 187/332 | 355 | 277 | 104 | 0 | 0 | 0 | 2.1 |
| stinkyscape-061 | Flak v5 | hard | base | cleared | 303/626 | 727 | 535 | 117 | 0 | 0 | 0 | 2.5 |
| stinkyscape-063 | Miner v2 | easy | base | cleared | 231/421 | 542 | 451 | 172 | 0 | 0 | 0 | -0.1 |
| stinkyscape-063 | Miner v2 | hard | base | cleared | 212/469 | 450 | 355 | 115 | 0 | 0 | 0 | 0.3 |
| stinkyscape-064 | Miner v3 | easy | base | cleared | 202/368 | 530 | 436 | 113 | 0 | 0 | 0 | 1.1 |
| stinkyscape-064 | Miner v3 | hard | base | cleared | 233/510 | 751 | 591 | 79 | 0 | 0 | 0 | 1.0 |
| stinkyscape-065 | Miner v4 | easy | base | cleared | 177/351 | 551 | 469 | 83 | 0 | 0 | 0 | 1.8 |
| stinkyscape-065 | Miner v4 | hard | base | cleared | 272/582 | 866 | 639 | 46 | 0 | 0 | 0 | 1.6 |
| stinkyscape-066 | Miner v5 | easy | base | cleared | 150/327 | 426 | 321 | 45 | 0 | 0 | 0 | 2.4 |
| stinkyscape-066 | Miner v5 | hard | base | cleared | 281/587 | 914 | 629 | 17 | 0 | 0 | 0 | 2.3 |
| stinkyscape-071 | A'Zoth | easy | base | cleared | 154/286 | 250 | 198 | 99 | 0 | 0 | 0 | 25.0 |
| stinkyscape-071 | A'Zoth | hard | base | cleared | 269/561 | 542 | 416 | 134 | 0 | 0 | 0 | 21.0 |
| stinkyscape-074 | Paragon-X | easy | base | cleared | 148/294 | 297 | 193 | 82 | 0 | 0 | 0 | 6.7 |
| stinkyscape-074 | Paragon-X | hard | base | cleared | 263/546 | 520 | 434 | 181 | 0 | 0 | 0 | 6.8 |
| stinkyscape-076 | Honey Badger | easy | base | cleared | 192/363 | 361 | 308 | 147 | 0 | 0 | 0 | 7.5 |
| stinkyscape-076 | Honey Badger | hard | base | cleared | 205/480 | 409 | 338 | 123 | 0 | 0 | 0 | 7.7 |
| stinkyscape-080 | Galaxy | easy | base | cleared | 166/314 | 475 | 255 | 24 | 0 | 0 | 0 | -6.5 |
| stinkyscape-080 | Galaxy | hard | base | cleared | 268/571 | 861 | 576 | 33 | 0 | 0 | 0 | 30291.8 |
| stinkyscape-082 | Phalanx | easy | base | cleared | 159/322 | 297 | 239 | 74 | 0 | 0 | 0 | 7.2 |
| stinkyscape-082 | Phalanx | hard | base | cleared | 259/513 | 742 | 534 | 57 | 0 | 0 | 0 | 6.3 |
| stinkyscape-083 | Halo | easy | base | cleared | 150/280 | 286 | 222 | 66 | 0 | 0 | 0 | 4.5 |
| stinkyscape-083 | Halo | hard | base | cleared | 225/524 | 656 | 486 | 33 | 0 | 0 | 0 | 4.9 |
| stinkyscape-084 | Power Cell | easy | base | cleared | 109/219 | 467 | 467 | 43 | 0 | 0 | 0 | -5.6 |
| stinkyscape-084 | Power Cell | hard | base | cleared | 104/242 | 327 | 327 | 28 | 0 | 0 | 0 | -1.3 |
| stinkyscape-085 | Prototype-X1 | easy | base | cleared | 158/299 | 355 | 244 | 57 | 0 | 0 | 0 | 6.4 |
| stinkyscape-085 | Prototype-X1 | hard | base | cleared | 212/512 | 571 | 473 | 65 | 0 | 0 | 0 | 7.4 |
| stinkyscape-086 | Prototype-X2 | easy | base | cleared | 175/317 | 542 | 158 | 36 | 0 | 0 | 0 | 2.0 |
| stinkyscape-086 | Prototype-X2 | hard | base | cleared | 253/520 | 598 | 222 | 28 | 0 | 0 | 0 | -0.7 |
| stinkyscape-087 | Siva 1 | easy | base | cleared | 243/459 | 353 | 274 | 58 | 0 | 0 | 0 | 5.4 |
| stinkyscape-087 | Siva 1 | hard | base | cleared | 289/630 | 433 | 347 | 39 | 0 | 0 | 0 | 5.6 |
| stinkyscape-088 | Siva 2 | easy | base | cleared | 276/492 | 551 | 493 | 216 | 0 | 0 | 0 | 13.7 |
| stinkyscape-088 | Siva 2 | hard | base | cleared | 306/674 | 586 | 447 | 179 | 0 | 0 | 0 | 15.6 |
| stinkyscape-089 | Spice | easy | base | cleared | 122/245 | 248 | 248 | 100 | 0 | 0 | 0 | -0.4 |
| stinkyscape-089 | Spice | hard | base | cleared | 109/235 | 233 | 233 | 65 | 0 | 0 | 0 | -0.4 |
| stinkyscape-093 | Patcher Nanobots | easy | base | cleared | 198/360 | 209 | 175 | 181 | 0 | 0 | 0 | 11.2 |
| stinkyscape-093 | Patcher Nanobots | hard | base | cleared | 227/505 | 325 | 269 | 178 | 0 | 0 | 0 | 10.8 |
| stinkyscape-095 | Faction: Vanguard Coalition | easy | base | cleared | 81/153 | 140 | 140 | 69 | 0 | 0 | 0 | 9.0 |
| stinkyscape-095 | Faction: Vanguard Coalition | hard | base | cleared | 109/227 | 298 | 298 | 56 | 0 | 0 | 0 | 6.4 |
| stinkyscape-096 | Ocean Tsunami | easy | base | cleared | 149/285 | 348 | 250 | 56 | 0 | 0 | 0 | 11.9 |
| stinkyscape-096 | Ocean Tsunami | hard | base | cleared | 265/576 | 795 | 660 | 54 | 0 | 0 | 0 | 3.7 |
| stinkyscape-104 | Reflective Plating | easy | base | cleared | 200/381 | 398 | 321 | 155 | 0 | 0 | 0 | 6.5 |
| stinkyscape-104 | Reflective Plating | hard | base | cleared | 236/501 | 598 | 492 | 122 | 0 | 0 | 0 | 4.8 |
| stinkyscape-109 | xxFreshJack | easy | base | cleared | 192/364 | 305 | 217 | 109 | 0 | 0 | 0 | 17.4 |
| stinkyscape-109 | xxFreshJack | hard | base | cleared | 289/632 | 545 | 449 | 188 | 0 | 0 | 0 | 17.7 |
| stinkyscape-114 | Arctic Citadel | easy | base | cleared | 181/311 | 369 | 284 | 74 | 0 | 0 | 0 | 16.6 |
| stinkyscape-114 | Arctic Citadel | hard | base | cleared | 233/546 | 625 | 436 | 67 | 0 | 0 | 0 | 30.9 |
| stinkyscape-115 | Scornboi | easy | base | cleared | 190/371 | 436 | 205 | 73 | 0 | 0 | 0 | 46.9 |
| stinkyscape-115 | Scornboi | hard | base | cleared | 322/666 | 807 | 505 | 111 | 0 | 0 | 0 | 31.7 |
| stinkyscape-117 | Core System | easy | base | cleared | 127/213 | 461 | 463 | 68 | 0 | 0 | 0 | 41.6 |
| stinkyscape-117 | Core System | hard | base | cleared | 136/298 | 474 | 473 | 19 | 0 | 0 | 0 | 37.3 |
| stinkyscape-118 | Secure System | easy | base | cleared | 92/190 | 332 | 331 | 40 | 0 | 0 | 0 | 32.7 |
| stinkyscape-118 | Secure System | hard | base | cleared | 149/313 | 517 | 517 | 17 | 0 | 0 | 0 | 43.6 |
| stinkyscape-119 | Unsecure System | easy | base | cleared | 92/170 | 244 | 244 | 58 | 0 | 0 | 0 | 0.1 |
| stinkyscape-119 | Unsecure System | hard | base | cleared | 151/329 | 341 | 341 | 118 | 0 | 0 | 0 | 15.6 |
| stinkyscape-120 | Wild System | easy | base | cleared | 91/184 | 249 | 249 | 51 | 0 | 0 | 0 | 41.3 |
| stinkyscape-120 | Wild System | hard | base | cleared | 143/322 | 425 | 425 | 34 | 0 | 0 | 0 | 7.1 |
| stinkyscape-121 | System Schematic | easy | base | cleared | 134/248 | 154 | 154 | 85 | 0 | 0 | 0 | 1.0 |
| stinkyscape-121 | System Schematic | hard | base | cleared | 93/227 | 109 | 109 | 42 | 0 | 0 | 0 | 1.3 |
| stinkyscape-124 | Jlockeed's Shipbuilding Grant | easy | base | cleared | 105/211 | 263 | 263 | 73 | 0 | 0 | 0 | 3.2 |
| stinkyscape-124 | Jlockeed's Shipbuilding Grant | hard | base | cleared | 135/308 | 433 | 433 | 55 | 0 | 0 | 0 | 2.8 |
| stinkyscape-125 | Jlockeed's Spacerace Propaganda | easy | base | cleared | 115/214 | 302 | 296 | 80 | 0 | 0 | 0 | 1.4 |
| stinkyscape-125 | Jlockeed's Spacerace Propaganda | hard | base | cleared | 129/275 | 379 | 366 | 46 | 0 | 0 | 0 | 1.6 |
| stinkyscape-126 | Jlockeed's Permanent Procurement Lobbying | easy | base | cleared | 94/191 | 260 | 260 | 56 | 0 | 0 | 0 | 1.2 |
| stinkyscape-126 | Jlockeed's Permanent Procurement Lobbying | hard | base | cleared | 129/277 | 363 | 363 | 47 | 0 | 0 | 0 | 1.0 |
| stinkyscape-127 | Jlockeed's Siege Tank | easy | base | cleared | 213/449 | 239 | 209 | 186 | 0 | 0 | 0 | 16.3 |
| stinkyscape-127 | Jlockeed's Siege Tank | hard | base | cleared | 315/664 | 341 | 279 | 273 | 0 | 0 | 0 | 17.4 |
| stinkyscape-128 | Jlockeed's Yamato Cannon | easy | base | cleared | 243/483 | 386 | 328 | 199 | 0 | 0 | 0 | 6.5 |
| stinkyscape-128 | Jlockeed's Yamato Cannon | hard | base | cleared | 231/527 | 378 | 299 | 170 | 0 | 0 | 0 | 7.0 |
| stinkyscape-130 | Jlockeed's Starship Retrofitting Program | easy | base | cleared | 132/258 | 345 | 345 | 93 | 0 | 0 | 0 | 0.6 |
| stinkyscape-130 | Jlockeed's Starship Retrofitting Program | hard | base | cleared | 126/279 | 345 | 345 | 66 | 0 | 0 | 0 | 0.2 |
| stinkyscape-131 | Spacerace Stimulus | easy | base | cleared | 126/224 | 171 | 169 | 119 | 0 | 0 | 0 | 6.9 |
| stinkyscape-131 | Spacerace Stimulus | hard | base | cleared | 106/250 | 143 | 142 | 96 | 0 | 0 | 0 | 7.5 |
| stinkyscape-132 | Defensive Crystal | easy | base | cleared | 193/381 | 254 | 211 | 165 | 0 | 0 | 0 | 9.4 |
| stinkyscape-132 | Defensive Crystal | hard | base | cleared | 219/494 | 378 | 324 | 169 | 0 | 0 | 0 | 9.9 |
| stinkyscape-133 | Ultra Capacitor | easy | base | cleared | 233/452 | 419 | 372 | 196 | 0 | 0 | 0 | 4.5 |
| stinkyscape-133 | Ultra Capacitor | hard | base | cleared | 193/478 | 336 | 273 | 124 | 0 | 0 | 0 | 5.0 |
| stinkyscape-134 | Felinor Containment Chamber | easy | base | cleared | 219/424 | 365 | 297 | 187 | 0 | 0 | 0 | 3.3 |
| stinkyscape-134 | Felinor Containment Chamber | hard | base | cleared | 216/475 | 352 | 282 | 163 | 0 | 0 | 0 | 3.8 |
| stinkyscape-141 | Fruit Launcher | easy | base | cleared | 187/364 | 503 | 417 | 108 | 0 | 0 | 0 | 1.0 |
| stinkyscape-141 | Fruit Launcher | hard | base | cleared | 259/554 | 903 | 701 | 91 | 0 | 0 | 0 | 1.0 |
| stinkyscape-144 | Prepare For Liftoff | easy | base | cleared | 114/219 | 143 | 139 | 30 | 0 | 0 | 0 | 1.5 |
| stinkyscape-144 | Prepare For Liftoff | hard | base | cleared | 109/241 | 133 | 127 | 25 | 0 | 0 | 0 | 0.7 |
| stinkyscape-145 | Fortify Flesh | easy | base | cleared | 63/135 | 151 | 152 | 12 | 0 | 0 | 0 | 27.1 |
| stinkyscape-145 | Fortify Flesh | hard | base | cleared | 127/293 | 352 | 352 | 27 | 0 | 0 | 0 | 17.8 |
| stinkyscape-150 | Call to Chaos (Stinky Edition) | easy | base | cleared | 104/235 | 215 | 215 | 62 | 0 | 0 | 0 | 9.1 |
| stinkyscape-150 | Call to Chaos (Stinky Edition) | hard | base | cleared | 165/352 | 347 | 347 | 79 | 0 | 0 | 0 | 9.1 |
| stinkyscape-151 | Call to Chaos (Scape Edition) | easy | base | cleared | 121/215 | 171 | 171 | 72 | 0 | 0 | 0 | 14.0 |
| stinkyscape-151 | Call to Chaos (Scape Edition) | hard | base | cleared | 153/340 | 308 | 308 | 87 | 0 | 0 | 0 | 12.6 |
| stinkyscape-152 | Sanctum | easy | base | cleared | 48/116 | 95 | 95 | 22 | 0 | 0 | 0 | 9.5 |
| stinkyscape-152 | Sanctum | hard | base | cleared | 120/274 | 334 | 327 | 50 | 0 | 0 | 0 | 7.5 |
| stinkyscape-155 | Comet Crash | easy | base | cleared | 92/176 | 308 | 308 | 51 | 0 | 0 | 0 | -0.3 |
| stinkyscape-155 | Comet Crash | hard | base | cleared | 120/269 | 425 | 416 | 28 | 0 | 0 | 0 | -0.0 |
| stinkyscape-156 | Catalyst Comet | easy | base | cleared | 123/237 | 347 | 347 | 105 | 0 | 0 | 0 | -0.1 |
| stinkyscape-156 | Catalyst Comet | hard | base | cleared | 106/225 | 312 | 312 | 60 | 0 | 0 | 0 | 0.2 |
| stinkyscape-157 | The Gauntlet | easy | base | cleared | 111/238 | 327 | 314 | 70 | 0 | 0 | 0 | 6.3 |
| stinkyscape-157 | The Gauntlet | hard | base | cleared | 117/254 | 367 | 349 | 58 | 0 | 0 | 0 | 1.1 |
| stinkyscape-158 | Stratos Swarm | easy | base | cleared | 84/139 | 69 | 69 | 55 | 0 | 0 | 0 | 42.2 |
| stinkyscape-158 | Stratos Swarm | hard | base | cleared | 131/281 | 146 | 146 | 113 | 0 | 0 | 0 | 44.8 |
| stinkyscape-159 | Faction: ETHICS | easy | base | cleared | 111/209 | 134 | 134 | 103 | 0 | 0 | 0 | 9.5 |
| stinkyscape-159 | Faction: ETHICS | hard | base | cleared | 85/236 | 127 | 127 | 65 | 0 | 0 | 0 | 7.1 |
| stinkyscape-161 | Faction: Pegasus | easy | base | cleared | 94/181 | 202 | 202 | 72 | 0 | 0 | 0 | 6.9 |
| stinkyscape-161 | Faction: Pegasus | hard | base | cleared | 104/230 | 284 | 284 | 42 | 0 | 0 | 0 | 4.9 |
| stinkyscape-162 | Faction: Star-Jumper Core | easy | base | cleared | 103/184 | 180 | 180 | 78 | 0 | 0 | 0 | 6.9 |
| stinkyscape-162 | Faction: Star-Jumper Core | hard | base | cleared | 108/239 | 278 | 278 | 58 | 0 | 0 | 0 | 5.2 |
| stinkyscape-163 | Swear Loyalities | easy | base | cleared | 114/231 | 101 | 101 | 105 | 0 | 0 | 0 | 0.4 |
| stinkyscape-163 | Swear Loyalities | hard | base | cleared | 116/244 | 106 | 106 | 102 | 0 | 0 | 0 | 0.6 |
| stinkyscape-164 | The Syndicate | easy | base | cleared | 111/228 | 127 | 119 | 40 | 0 | 0 | 0 | 0.2 |
| stinkyscape-164 | The Syndicate | hard | base | cleared | 106/232 | 108 | 105 | 52 | 0 | 0 | 0 | 0.8 |
| stinkyscape-165 | The Trade Union | easy | base | cleared | 93/195 | 109 | 108 | 42 | 0 | 0 | 0 | 0.2 |
| stinkyscape-165 | The Trade Union | hard | base | cleared | 111/236 | 112 | 108 | 59 | 0 | 0 | 0 | 0.5 |
| stinkyscape-166 | Scanning Probe Mk1 | easy | base | cleared | 103/195 | 360 | 360 | 67 | 0 | 0 | 0 | -0.1 |
| stinkyscape-166 | Scanning Probe Mk1 | hard | base | cleared | 96/214 | 326 | 326 | 48 | 0 | 0 | 0 | 0.6 |
| stinkyscape-167 | Scanning Probe Mk2 | easy | base | cleared | 80/164 | 272 | 272 | 37 | 0 | 0 | 0 | 0.1 |
| stinkyscape-167 | Scanning Probe Mk2 | hard | base | cleared | 133/273 | 519 | 518 | 28 | 0 | 0 | 0 | 0.5 |
| stinkyscape-171 | Large Asteroid | easy | base | cleared | 212/413 | 629 | 525 | 124 | 0 | 0 | 0 | 7.6 |
| stinkyscape-171 | Large Asteroid | hard | base | cleared | 268/595 | 820 | 663 | 80 | 0 | 0 | 0 | 6.4 |
| stinkyscape-172 | Huge Asteroid | easy | base | cleared | 153/304 | 324 | 248 | 59 | 0 | 0 | 0 | 7.5 |
| stinkyscape-172 | Huge Asteroid | hard | base | cleared | 260/565 | 778 | 564 | 50 | 0 | 0 | 0 | 7.6 |
| stinkyscape-173 | Abandoned Spice Tower | easy | base | cleared | 170/308 | 379 | 295 | 120 | 0 | 0 | 0 | 6.4 |
| stinkyscape-173 | Abandoned Spice Tower | hard | base | cleared | 239/520 | 567 | 423 | 112 | 0 | 0 | 0 | 6.5 |
| stinkyscape-174 | Aerelret | easy | base | cleared | 81/154 | 184 | 172 | 32 | 0 | 0 | 0 | 2.0 |
| stinkyscape-174 | Aerelret | hard | base | cleared | 129/285 | 360 | 346 | 28 | 0 | 0 | 0 | 2.6 |
| stinkyscape-175 | Frost Module | easy | base | cleared | 232/439 | 371 | 313 | 205 | 0 | 0 | 0 | 3.4 |
| stinkyscape-175 | Frost Module | hard | base | cleared | 221/469 | 355 | 270 | 154 | 0 | 0 | 0 | 3.3 |
| stinkyscapeplus-001 | Starfruit | easy | base | cleared | 167/293 | 467 | 467 | 132 | 0 | 0 | 0 | 3.1 |
| stinkyscapeplus-001 | Starfruit | hard | base | cleared | 146/308 | 394 | 394 | 86 | 0 | 0 | 0 | 4.0 |
| stinkyscapeplus-004 | Profe Yates | easy | base | cleared | 256/578 | 593 | 475 | 202 | 0 | 0 | 0 | 9.5 |
| stinkyscapeplus-004 | Profe Yates | hard | base | cleared | 314/710 | 711 | 539 | 225 | 0 | 0 | 0 | 10.3 |
| stinkyscapeplus-005 | Dingleberry Droat | easy | base | cleared | 236/439 | 576 | 488 | 159 | 0 | 0 | 0 | 5.5 |
| stinkyscapeplus-005 | Dingleberry Droat | hard | base | cleared | 273/617 | 798 | 642 | 114 | 0 | 0 | 0 | 5.5 |
| stinkyscapeplus-008 | Drone Assembler v3 | easy | base | cleared | 220/403 | 362 | 293 | 180 | 0 | 0 | 0 | 0.7 |
| stinkyscapeplus-008 | Drone Assembler v3 | hard | base | cleared | 240/523 | 597 | 473 | 137 | 0 | 0 | 0 | 0.6 |
| stinkyscapeplus-009 | Drone Assembler v4 | easy | base | cleared | 231/442 | 420 | 354 | 163 | 0 | 0 | 0 | 1.2 |
| stinkyscapeplus-009 | Drone Assembler v4 | hard | base | cleared | 269/637 | 588 | 446 | 139 | 0 | 0 | 0 | 1.4 |
| stinkyscapeplus-013 | Shield Repairer | easy | base | cleared | 193/345 | 354 | 304 | 151 | 0 | 0 | 0 | 4.3 |
| stinkyscapeplus-013 | Shield Repairer | hard | base | cleared | 234/517 | 522 | 411 | 129 | 0 | 0 | 0 | 4.4 |
| stinkyscapeplus-015 | Hull Repairer mk2 | easy | base | cleared | 205/360 | 466 | 407 | 141 | 0 | 0 | 0 | 4.2 |
| stinkyscapeplus-015 | Hull Repairer mk2 | hard | base | cleared | 229/476 | 665 | 542 | 103 | 0 | 0 | 0 | 5.6 |
| stinkyscapeplus-023 | GIGA-ify | easy | base | cleared | 97/185 | 426 | 426 | 29 | 0 | 0 | 0 | -1.1 |
| stinkyscapeplus-023 | GIGA-ify | hard | base | cleared | 124/252 | 442 | 442 | 29 | 0 | 0 | 0 | 0.5 |
| stinkyscapeplus-024 | Forced Propagation | easy | base | cleared | 108/198 | 442 | 442 | 52 | 0 | 0 | 0 | -1.3 |
| stinkyscapeplus-024 | Forced Propagation | hard | base | cleared | 111/252 | 385 | 385 | 39 | 0 | 0 | 0 | 0.0 |
| stinkyscapeplus-026 | Jlockeed’s Forever War | easy | base | cleared | 137/255 | 249 | 249 | 126 | 0 | 0 | 0 | 1.6 |
| stinkyscapeplus-026 | Jlockeed’s Forever War | hard | base | cleared | 125/298 | 221 | 221 | 94 | 0 | 0 | 0 | 1.5 |
| stinkyscapeplus-030 | Yeah I’m a War Hero | easy | base | cleared | 107/212 | 287 | 266 | 73 | 0 | 0 | 0 | 0.4 |
| stinkyscapeplus-030 | Yeah I’m a War Hero | hard | base | cleared | 121/246 | 347 | 333 | 65 | 0 | 0 | 0 | 0.8 |
| stinkyscapeplus-032 | Z-Patch | easy | base | cleared | 98/171 | 169 | 169 | 34 | 0 | 0 | 0 | 2.0 |
| stinkyscapeplus-032 | Z-Patch | hard | base | cleared | 126/299 | 197 | 197 | 25 | 0 | 0 | 0 | 8.4 |
| stinkyscapeplus-033 | True Conservative | easy | base | cleared | 168/323 | 369 | 301 | 57 | 0 | 0 | 0 | 11.4 |
| stinkyscapeplus-033 | True Conservative | hard | base | cleared | 242/550 | 672 | 469 | 54 | 0 | 0 | 0 | 14.9 |
| stinkyscapeplus-038 | Arc | easy | base | cleared | 207/365 | 393 | 335 | 157 | 0 | 0 | 0 | 3.1 |
| stinkyscapeplus-038 | Arc | hard | base | cleared | 237/535 | 648 | 507 | 106 | 0 | 0 | 0 | 3.4 |
| stinkyscapeplus-040 | Mender | easy | base | cleared | 103/211 | 95 | 93 | 98 | 0 | 0 | 0 | 0.4 |
| stinkyscapeplus-040 | Mender | hard | base | cleared | 96/225 | 95 | 89 | 84 | 0 | 0 | 0 | 0.6 |
| stinkyscapeplus-041 | Mend Projector | easy | base | cleared | 102/210 | 307 | 299 | 65 | 0 | 0 | 0 | 2.1 |
| stinkyscapeplus-041 | Mend Projector | hard | base | cleared | 157/363 | 440 | 434 | 66 | 0 | 0 | 0 | 2.1 |
| stinkyscapeplus-043 | Segment | easy | base | cleared | 106/175 | 357 | 353 | 57 | 0 | 0 | 0 | 1.0 |
| stinkyscapeplus-043 | Segment | hard | base | cleared | 119/273 | 438 | 428 | 55 | 0 | 0 | 0 | 1.5 |
| stinkyscapeplus-046 | Phase Fabric | easy | base | cleared | 175/318 | 367 | 367 | 161 | 0 | 0 | 0 | 3.4 |
| stinkyscapeplus-046 | Phase Fabric | hard | base | cleared | 148/320 | 274 | 274 | 143 | 0 | 0 | 0 | 3.6 |
| stinkyscapeplus-049 | Metaglass | easy | base | cleared | 160/335 | 615 | 615 | 85 | 0 | 0 | 0 | -2.3 |
| stinkyscapeplus-049 | Metaglass | hard | base | cleared | 177/356 | 554 | 554 | 62 | 0 | 0 | 0 | -0.7 |
| stinkyscapeplus-050 | Graphite | easy | base | cleared | 152/326 | 452 | 452 | 115 | 0 | 0 | 0 | 1.4 |
| stinkyscapeplus-050 | Graphite | hard | base | cleared | 147/316 | 391 | 391 | 85 | 0 | 0 | 0 | 2.4 |
| stinkyscapeplus-052 | Airblast Drill | easy | base | cleared | 85/152 | 258 | 136 | 6 | 0 | 0 | 0 | -29.7 |
| stinkyscapeplus-052 | Airblast Drill | hard | base | cleared | 138/311 | 477 | 265 | 1 | 0 | 0 | 0 | -4.0 |
| stinkyscapeplus-053 | Laser Drill | easy | base | cleared | 89/157 | 275 | 131 | 13 | 0 | 0 | 0 | 3.1 |
| stinkyscapeplus-053 | Laser Drill | hard | base | cleared | 119/280 | 433 | 179 | 2 | 0 | 0 | 0 | 6.9 |
| stinkyscapeplus-054 | Pneumatic Drill | easy | base | cleared | 109/205 | 365 | 340 | 62 | 0 | 0 | 0 | 1.6 |
| stinkyscapeplus-054 | Pneumatic Drill | hard | base | cleared | 106/250 | 345 | 342 | 44 | 0 | 0 | 0 | 1.2 |
| stinkyscapeplus-056 | Impact Reactor | easy | base | cleared | 105/212 | 496 | 245 | 7 | 0 | 0 | 0 | -0.6 |
| stinkyscapeplus-056 | Impact Reactor | hard | base | cleared | 140/316 | 577 | 286 | 2 | 0 | 0 | 0 | -1.2 |
| stinkyscapeplus-057 | Phase Weaver | easy | base | cleared | 104/239 | 96 | 93 | 100 | 0 | 0 | 0 | 0.8 |
| stinkyscapeplus-057 | Phase Weaver | hard | base | cleared | 107/235 | 111 | 106 | 97 | 0 | 0 | 0 | 0.5 |
| stinkyscapeplus-060 | Radi | easy | base | cleared | 165/342 | 394 | 317 | 107 | 0 | 0 | 0 | 1.7 |
| stinkyscapeplus-060 | Radi | hard | base | cleared | 225/508 | 703 | 541 | 71 | 0 | 0 | 0 | 2.9 |
| stinkyscapeplus-063 | Overdrive Dome | easy | base | cleared | 66/135 | 190 | 167 | 11 | 0 | 0 | 0 | 2.4 |
| stinkyscapeplus-063 | Overdrive Dome | hard | base | cleared | 140/290 | 415 | 399 | 19 | 0 | 0 | 0 | 2.3 |
| stinkyscapeplus-068 | Shadiris Visions | easy | base | cleared | 101/191 | 243 | 243 | 81 | 0 | 0 | 0 | -1.2 |
| stinkyscapeplus-068 | Shadiris Visions | hard | base | cleared | 118/260 | 288 | 288 | 72 | 0 | 0 | 0 | -1.2 |
| stinkyscapeplus-069 | Exotic Jellybean | easy | base | cleared | 135/263 | 548 | 548 | 43 | 0 | 0 | 0 | 0.0 |
| stinkyscapeplus-069 | Exotic Jellybean | hard | base | cleared | 143/341 | 507 | 507 | 29 | 0 | 0 | 0 | 0.1 |
| stinkyscapeplus-072 | Foreshadow | easy | base | cleared | 174/304 | 556 | 86 | 5 | 0 | 0 | 0 | 20.5 |
| stinkyscapeplus-072 | Foreshadow | hard | base | cleared | 240/534 | 574 | 182 | 11 | 0 | 0 | 0 | 15.1 |
| stinkyscapeplus-073 | Ripple | easy | base | cleared | 206/392 | 302 | 296 | 184 | 0 | 0 | 0 | 14.5 |
| stinkyscapeplus-073 | Ripple | hard | base | cleared | 269/584 | 395 | 391 | 213 | 0 | 0 | 0 | 14.7 |
| stinkyscapeplus-077 | Black Hole | easy | base | cleared | 109/202 | 301 | 301 | 35 | 0 | 0 | 0 | 31.8 |
| stinkyscapeplus-077 | Black Hole | hard | base | cleared | 167/368 | 536 | 536 | 18 | 0 | 0 | 0 | 8.0 |
| stinkyscapeplus-079 | A Glimpse into the Endless Sea | easy | base | cleared | 108/200 | 119 | 119 | 76 | 0 | 0 | 0 | 9.2 |
| stinkyscapeplus-079 | A Glimpse into the Endless Sea | hard | base | cleared | 167/345 | 277 | 277 | 111 | 0 | 0 | 0 | -8.8 |
| stinkyscapeplus-085 | Body Snatch | easy | base | cleared | 85/170 | 207 | 207 | 61 | 0 | 0 | 0 | 18.3 |
| stinkyscapeplus-085 | Body Snatch | hard | base | cleared | 107/250 | 306 | 306 | 52 | 0 | 0 | 0 | 11.7 |
| stinkyscapeplus-087 | Frontier Flora | easy | base | cleared | 158/282 | 383 | 369 | 124 | 0 | 0 | 0 | 0.9 |
| stinkyscapeplus-087 | Frontier Flora | hard | base | cleared | 124/286 | 303 | 296 | 78 | 0 | 0 | 0 | 0.8 |
| stinkyscapeplus-098 | Doomed Ship | easy | base | cleared | 96/172 | 275 | 275 | 60 | 0 | 0 | 0 | 2.0 |
| stinkyscapeplus-098 | Doomed Ship | hard | base | cleared | 99/228 | 319 | 319 | 38 | 0 | 0 | 0 | 2.2 |
| stinkyscapeplus-103 | Transcend | easy | base | cleared | 115/208 | 265 | 265 | 104 | 0 | 0 | 0 | 12.9 |
| stinkyscapeplus-103 | Transcend | hard | base | cleared | 126/262 | 224 | 224 | 99 | 0 | 0 | 0 | 13.8 |
| stinkyscapeplus-106 | Spectre | easy | base | cleared | 174/327 | 308 | 147 | 87 | 0 | 0 | 0 | 21.6 |
| stinkyscapeplus-106 | Spectre | hard | base | cleared | 260/620 | 563 | 246 | 106 | 0 | 0 | 0 | 22.9 |
| stinkyscapeplus-109 | Lancer | easy | base | cleared | 226/410 | 439 | 435 | 181 | 0 | 0 | 0 | 4.2 |
| stinkyscapeplus-109 | Lancer | hard | base | cleared | 223/518 | 546 | 539 | 134 | 0 | 0 | 0 | 4.3 |
| stinkyscapeplus-111 | Mace | easy | base | cleared | 230/436 | 384 | 314 | 188 | 0 | 0 | 0 | 6.8 |
| stinkyscapeplus-111 | Mace | hard | base | cleared | 227/501 | 359 | 300 | 165 | 0 | 0 | 0 | 7.1 |
| stinkyscapeplus-113 | Scepter | easy | base | cleared | 241/435 | 713 | 422 | 125 | 0 | 0 | 0 | 16.8 |
| stinkyscapeplus-113 | Scepter | hard | base | cleared | 273/593 | 693 | 506 | 161 | 0 | 0 | 0 | 17.3 |
| stinkyscapeplus-119 | Eclipse | easy | base | cleared | 188/336 | 517 | 214 | 60 | 0 | 0 | 0 | 18.6 |
| stinkyscapeplus-119 | Eclipse | hard | base | cleared | 241/527 | 598 | 357 | 108 | 0 | 0 | 0 | 19.0 |
| stinkyscapeplus-123 | Oct | easy | base | cleared | 163/347 | 497 | 244 | 47 | 0 | 0 | 0 | 23.0 |
| stinkyscapeplus-123 | Oct | hard | base | cleared | 268/576 | 775 | 508 | 66 | 0 | 0 | 0 | 7.4 |
| stinkyscapeplus-125 | Omura | easy | base | cleared | 145/304 | 255 | 101 | 86 | 0 | 0 | 0 | 24.5 |
| stinkyscapeplus-125 | Omura | hard | base | cleared | 266/580 | 511 | 280 | 181 | 0 | 0 | 0 | 18.3 |
| stinkyscapeplus-126 | Vela | easy | base | cleared | 222/417 | 872 | 602 | 66 | 0 | 0 | 0 | -1.5 |
| stinkyscapeplus-126 | Vela | hard | base | cleared | 272/606 | 878 | 644 | 50 | 0 | 0 | 0 | 0.2 |
| stinkyscapeplus-129 | Multiplicative Reconstructor | easy | base | cleared | 84/167 | 236 | 234 | 60 | 0 | 0 | 0 | -1.7 |
| stinkyscapeplus-129 | Multiplicative Reconstructor | hard | base | cleared | 112/241 | 331 | 329 | 48 | 0 | 0 | 0 | 1.7 |
| stinkyscapeplus-130 | Exponential Reconstructor | easy | base | cleared | 83/150 | 252 | 223 | 23 | 0 | 0 | 0 | 1.9 |
| stinkyscapeplus-130 | Exponential Reconstructor | hard | base | cleared | 155/326 | 450 | 418 | 25 | 0 | 0 | 0 | -2.1 |
| stinkyscapeplus-131 | Tetrative Reconstructor | easy | base | cleared | 86/160 | 262 | 137 | 3 | 0 | 0 | 0 | -3.1 |
| stinkyscapeplus-131 | Tetrative Reconstructor | hard | base | cleared | 145/304 | 490 | 311 | 6 | 0 | 0 | 0 | -22.0 |
| stinkyscapeplus-133 | Great Paperclip Theory III | easy | base | cleared | 86/155 | 259 | 259 | 28 | 0 | 0 | 0 | -1.3 |
| stinkyscapeplus-133 | Great Paperclip Theory III | hard | base | cleared | 127/275 | 338 | 338 | 25 | 0 | 0 | 0 | -0.8 |
| stinkyscapeplus-136 | Crawler Assembly Line | easy | base | cleared | 87/165 | 199 | 192 | 52 | 0 | 0 | 0 | 2.0 |
| stinkyscapeplus-136 | Crawler Assembly Line | hard | base | cleared | 156/334 | 388 | 373 | 61 | 0 | 0 | 0 | 2.0 |
| stinkyscapeplus-138 | Delta Definition | easy | base | cleared | 96/176 | 242 | 242 | 72 | 0 | 0 | 0 | 3.7 |
| stinkyscapeplus-138 | Delta Definition | hard | base | cleared | 113/246 | 358 | 358 | 49 | 0 | 0 | 0 | 3.0 |
| stinkyscapeplus-140 | Automa Reintegration Plan | easy | base | cleared | 98/222 | 252 | 252 | 81 | 0 | 0 | 0 | -1.3 |
| stinkyscapeplus-140 | Automa Reintegration Plan | hard | base | cleared | 119/249 | 301 | 301 | 81 | 0 | 0 | 0 | -1.3 |
| stinkyscapeplus-142 | Call to Chaos (Mindustrial-Swarm Edition) | easy | base | cleared | 120/209 | 216 | 216 | 74 | 0 | 0 | 0 | 10.0 |
| stinkyscapeplus-142 | Call to Chaos (Mindustrial-Swarm Edition) | hard | base | cleared | 159/353 | 332 | 332 | 95 | 0 | 0 | 0 | 9.8 |
| stinkyscapeplus-143 | Fire Beam v3 | easy | base | cleared | 224/430 | 630 | 512 | 130 | 0 | 0 | 0 | 0.5 |
| stinkyscapeplus-143 | Fire Beam v3 | hard | base | cleared | 249/555 | 817 | 616 | 66 | 0 | 0 | 0 | 1.0 |
| stinkyscapeplus-144 | Fire Beam v4 | easy | base | cleared | 200/393 | 534 | 407 | 120 | 0 | 0 | 0 | 1.4 |
| stinkyscapeplus-144 | Fire Beam v4 | hard | base | cleared | 300/644 | 891 | 656 | 78 | 0 | 0 | 0 | 1.5 |
| stinkyscapeplus-145 | Fire Beam v5 | easy | base | cleared | 191/343 | 341 | 247 | 95 | 0 | 0 | 0 | 1.9 |
| stinkyscapeplus-145 | Fire Beam v5 | hard | base | cleared | 277/585 | 733 | 511 | 72 | 0 | 0 | 0 | 1.6 |
| stinkyscapeplus-146 | Scourge Mortar v3 | easy | base | cleared | 200/423 | 512 | 434 | 139 | 0 | 0 | 0 | 0.6 |
| stinkyscapeplus-146 | Scourge Mortar v3 | hard | base | cleared | 254/558 | 793 | 587 | 103 | 0 | 0 | 0 | 0.9 |
| stinkyscapeplus-147 | Scourge Mortar v4 | easy | base | cleared | 221/408 | 635 | 506 | 131 | 0 | 0 | 0 | 1.3 |
| stinkyscapeplus-147 | Scourge Mortar v4 | hard | base | cleared | 272/627 | 837 | 585 | 82 | 0 | 0 | 0 | 1.4 |
| stinkyscapeplus-148 | Scourge Mortar v5 | easy | base | cleared | 173/329 | 334 | 252 | 81 | 0 | 0 | 0 | 1.8 |
| stinkyscapeplus-148 | Scourge Mortar v5 | hard | base | cleared | 311/639 | 855 | 583 | 66 | 0 | 0 | 0 | 2.9 |
| stinkyscapeplus-150 | Scorching Rounds | easy | base | cleared | 172/333 | 449 | 365 | 110 | 0 | 0 | 0 | 4.7 |
| stinkyscapeplus-150 | Scorching Rounds | hard | base | cleared | 223/498 | 592 | 477 | 93 | 0 | 0 | 0 | 5.1 |
| stinkyscapeplus-151 | Volcanic Rounds | easy | base | cleared | 154/295 | 414 | 324 | 56 | 0 | 0 | 0 | 4.7 |
| stinkyscapeplus-151 | Volcanic Rounds | hard | base | cleared | 233/500 | 679 | 469 | 38 | 0 | 0 | 0 | 6.4 |
| stinkyscapeplus-153 | Noxious Rounds | easy | base | cleared | 206/391 | 762 | 609 | 70 | 0 | 0 | 0 | 3.7 |
| stinkyscapeplus-153 | Noxious Rounds | hard | base | cleared | 223/529 | 819 | 607 | 28 | 0 | 0 | 0 | 2.8 |
| stinkyscapeplus-155 | The Far Vagabond | easy | base | cleared | 71/140 | 134 | 134 | 26 | 0 | 0 | 0 | 6.9 |
| stinkyscapeplus-155 | The Far Vagabond | hard | base | cleared | 139/275 | 386 | 386 | 44 | 0 | 0 | 0 | 6.0 |
| stinkyscapeplus-156 | Horizon Breaker | easy | base | cleared | 179/325 | 341 | 257 | 94 | 0 | 0 | 0 | 11.0 |
| stinkyscapeplus-156 | Horizon Breaker | hard | base | cleared | 258/558 | 630 | 451 | 112 | 0 | 0 | 0 | 11.7 |
| stinkyscapeplus-160 | Icarus | easy | base | cleared | 151/278 | 451 | 245 | 32 | 0 | 0 | 0 | 3.9 |
| stinkyscapeplus-160 | Icarus | hard | base | cleared | 252/531 | 720 | 464 | 52 | 0 | 0 | 0 | 6.6 |
| stinkyscapeplus-165 | Shadiris | easy | base | cleared | 168/326 | 218 | 167 | 112 | 0 | 0 | 0 | 16.1 |
| stinkyscapeplus-165 | Shadiris | hard | base | cleared | 237/501 | 494 | 356 | 101 | 0 | 0 | 0 | 15.9 |
| stinkyscapeplus-166 | Mindustrial Revolution | easy | base | cleared | 88/170 | 199 | 199 | 62 | 0 | 0 | 0 | 2.4 |
| stinkyscapeplus-166 | Mindustrial Revolution | hard | base | cleared | 103/226 | 266 | 266 | 60 | 0 | 0 | 0 | 2.9 |
| traininggrounds-007 | Chosen by the Moon | easy | base | cleared | 223/427 | 279 | 241 | 207 | 0 | 0 | 0 | 12.5 |
| traininggrounds-007 | Chosen by the Moon | hard | base | cleared | 224/479 | 279 | 233 | 182 | 0 | 0 | 0 | 12.5 |
| traininggrounds-012 | Fortify | easy | base | cleared | 110/212 | 218 | 218 | 131 | 0 | 0 | 0 | 9.2 |
| traininggrounds-012 | Fortify | hard | base | cleared | 109/252 | 150 | 150 | 86 | 0 | 0 | 0 | 7.8 |
| traininggrounds-013 | Explosive Runes | easy | base | cleared | 103/209 | 172 | 168 | 92 | 0 | 0 | 0 | 0.8 |
| traininggrounds-013 | Explosive Runes | hard | base | cleared | 86/236 | 182 | 179 | 58 | 0 | 0 | 0 | 1.0 |
| traininggrounds-016 | Celestial Caller | easy | base | cleared | 164/301 | 197 | 158 | 120 | 0 | 0 | 0 | 41.6 |
| traininggrounds-016 | Celestial Caller | hard | base | cleared | 227/516 | 334 | 250 | 140 | 0 | 0 | 0 | 34.3 |
| traininggrounds-017 | Flaming Felinor | easy | base | cleared | 195/416 | 314 | 225 | 156 | 0 | 0 | 0 | 20.1 |
| traininggrounds-017 | Flaming Felinor | hard | base | cleared | 231/527 | 433 | 339 | 146 | 0 | 0 | 0 | 13.2 |
| traininggrounds-019 | Extra Rations | easy | base | cleared | 96/179 | 204 | 198 | 86 | 0 | 0 | 0 | 0.6 |
| traininggrounds-019 | Extra Rations | hard | base | cleared | 115/237 | 207 | 199 | 90 | 0 | 0 | 0 | 1.0 |
| traininggrounds-020 | Recruitment Posters | easy | base | cleared | 94/182 | 302 | 274 | 56 | 0 | 0 | 0 | 1.2 |
| traininggrounds-020 | Recruitment Posters | hard | base | cleared | 97/218 | 292 | 285 | 40 | 0 | 0 | 0 | 1.5 |
| traininggrounds-031 | Level Up! | easy | base | cleared | 92/192 | 311 | 299 | 45 | 0 | 0 | 0 | 5.6 |
| traininggrounds-031 | Level Up! | hard | base | cleared | 128/305 | 411 | 399 | 44 | 0 | 0 | 0 | 1.2 |
| traininggrounds-035 | Corpse Recycler | easy | base | cleared | 96/191 | 279 | 271 | 57 | 0 | 0 | 0 | 2.1 |
| traininggrounds-035 | Corpse Recycler | hard | base | cleared | 143/309 | 413 | 391 | 55 | 0 | 0 | 0 | 2.2 |
| traininggrounds-036 | Training Dummy | easy | base | cleared | 213/424 | 513 | 441 | 161 | 0 | 0 | 0 | 1.5 |
| traininggrounds-036 | Training Dummy | hard | base | cleared | 225/490 | 600 | 452 | 132 | 0 | 0 | 0 | 2.0 |
| traininggrounds-037 | Seance | easy | base | cleared | 100/182 | 309 | 309 | 64 | 0 | 0 | 0 | 3.4 |
| traininggrounds-037 | Seance | hard | base | cleared | 123/250 | 359 | 359 | 58 | 0 | 0 | 0 | 3.9 |
| traininggrounds-038 | Tutorial Master | easy | base | cleared | 161/316 | 175 | 135 | 105 | 0 | 0 | 0 | 25.8 |
| traininggrounds-038 | Tutorial Master | hard | base | cleared | 291/618 | 446 | 349 | 191 | 0 | 0 | 0 | 25.2 |
| traininggrounds-046 | Legacy: Hit Job | easy | base | cleared | 104/180 | 228 | 228 | 77 | 0 | 0 | 0 | 18.9 |
| traininggrounds-046 | Legacy: Hit Job | hard | base | cleared | 149/310 | 336 | 336 | 96 | 0 | 0 | 0 | 14.4 |
| traininggrounds-049 | Legacy: Reoccurring Dream | easy | base | cleared | 129/233 | 728 | 728 | 226 | 0 | 0 | 0 | -0.4 |
| traininggrounds-049 | Legacy: Reoccurring Dream | hard | base | cleared | 123/263 | 526 | 526 | 159 | 0 | 0 | 0 | 1.3 |
| traininggrounds-050 | Legacy: Glowy Jelly Bean | easy | base | cleared | 130/241 | 379 | 379 | 88 | 0 | 0 | 0 | 4.9 |
| traininggrounds-050 | Legacy: Glowy Jelly Bean | hard | base | cleared | 146/299 | 389 | 389 | 83 | 0 | 0 | 0 | 7.9 |
| traininggrounds-053 | Legacy: Prejudiced Postdoc | easy | base | cleared | 200/392 | 383 | 312 | 161 | 0 | 0 | 0 | 13.6 |
| traininggrounds-053 | Legacy: Prejudiced Postdoc | hard | base | cleared | 242/560 | 616 | 493 | 166 | 0 | 0 | 0 | 16.0 |
| traininggrounds-055 | Legacy: Reminisce | easy | base | cleared | 110/204 | 309 | 309 | 85 | 0 | 0 | 0 | -1.3 |
| traininggrounds-055 | Legacy: Reminisce | hard | base | cleared | 102/255 | 280 | 280 | 56 | 0 | 0 | 0 | -1.3 |
| traininggrounds-060 | Legacy: Rush Token Farm | easy | base | cleared | 90/162 | 113 | 105 | 76 | 0 | 0 | 0 | 1.4 |
| traininggrounds-060 | Legacy: Rush Token Farm | hard | base | cleared | 114/255 | 216 | 211 | 79 | 0 | 0 | 0 | 1.4 |

### Suspects (an error or timeout in a pass-2 game that also dealt this at-risk card as filler)

- suspect: classic-074 Corpse Plantation: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: classicplus-022 Blood Moon: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: classicplus-030 Felinor Fuser: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: core-093 Combo-Index: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: infiniteassault-023 The Generator: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: meditative-069 The Maestro: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: stinkyscape-036 Drone Core X: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: stinkyscape-083 Halo: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: stinkyscape-084 Power Cell: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: stinkyscape-087 Siva 1: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: stinkyscape-088 Siva 2: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: stinkyscape-096 Ocean Tsunami: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: stinkyscapeplus-004 Profe Yates: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: stinkyscapeplus-056 Impact Reactor: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: traininggrounds-017 Flaming Felinor: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-161:2 (forced infiniteassault-161 Defense in Depth); banned only if its own games repeat it
- suspect: classic-026 Rapid Draw: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: infiniteassault-052 Thermal Paste: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: infiniteassault-093 Gestalt: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: infiniteassault-100 Iterator: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: infiniteassault-106 Checkpoint Daemon: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: infiniteassault-116 Groundhog Day: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: infiniteassault-135 Mana Leech: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: infiniteassault-178 Dataset Thief: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: infiniteassault-214 Emergent Mind: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscape-008 Conjure Rosemary Triscuit: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscape-115 Scornboi: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscape-119 Unsecure System: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscape-124 Jlockeed's Shipbuilding Grant: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscape-166 Scanning Probe Mk1: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscapeplus-004 Profe Yates: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscapeplus-041 Mend Projector: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscapeplus-043 Segment: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscapeplus-087 Frontier Flora: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: stinkyscapeplus-111 Mace: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: traininggrounds-007 Chosen by the Moon: 1 error(s), 0 timeout(s) in sweep2:easy:infiniteassault-035:7 (forced infiniteassault-035 Fata Morgana); banned only if its own games repeat it
- suspect: classicplus-073 Call to Chaos (Classic+ Edition): 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: infiniteassault-053 Overvolt Grunt: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: infiniteassault-086 Replica Hatchery: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: infiniteassault-143 Zero-Click: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: infiniteassault-194 Compute Shard: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: infiniteassault-207 Recursive Self-Improvement: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscape-004 Olivian Expanse: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscape-085 Prototype-X1: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscape-087 Siva 1: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscape-088 Siva 2: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscape-109 xxFreshJack: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscape-115 Scornboi: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscape-141 Fruit Launcher: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscapeplus-005 Dingleberry Droat: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscapeplus-054 Pneumatic Drill: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: stinkyscapeplus-156 Horizon Breaker: 0 error(s), 5 timeout(s) in sweep2:easy:infiniteassault-048:3 (forced infiniteassault-048 Ego Death); banned only if its own games repeat it
- suspect: classic-015 Nose Hunter: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: classicplus-029 Portal to the Past: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: classicplus-033 Ivory Tower: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: classicplus-041 KY's Constant: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: core-007 Jewelosco Scarab: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: infiniteassault-010 Assault Carrier: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: infiniteassault-011 Quickgen Turret: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: infiniteassault-103 Undo: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: infiniteassault-135 Mana Leech: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: infiniteassault-160 Zero Trust: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: infiniteassault-194 Compute Shard: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: infiniteassault-213 The Long Game: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: meditative-022 Mind Games: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: stinkyscape-074 Paragon-X: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: stinkyscape-085 Prototype-X1: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: stinkyscape-104 Reflective Plating: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: stinkyscape-155 Comet Crash: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: stinkyscape-171 Large Asteroid: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: stinkyscape-173 Abandoned Spice Tower: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: stinkyscapeplus-009 Drone Assembler v4: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: stinkyscapeplus-030 Yeah I’m a War Hero: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: stinkyscapeplus-113 Scepter: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: traininggrounds-016 Celestial Caller: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it
- suspect: traininggrounds-017 Flaming Felinor: 0 error(s), 1 timeout(s) in sweep2:hard:infiniteassault-123:18 (forced infiniteassault-123 Cost Injection); banned only if its own games repeat it

### Unswept (never affordable at any tier: no evidence either way)

None.

### For crates/ai/src/shadow_ban.rs

Header line:

// Sweep of record: 2026-11-25 (UTC), `cargo jackioh sweep`, pass 1 over 240 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 139 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), each card also on its Radiant face (`sweep:<tier>:<id>:radiant:<n>`, `sweep2:<tier>:<id>:radiant:<n>`), budget AI_GATE_BUDGET {"nodes":1200,"lethalNodes":300,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.

SHADOW_BAN entries:

```rust
    ("infiniteassault-029", "timeout: easy: 2 decision(s) over 2000 ms or game(s) past 600 actions"),
    ("infiniteassault-035", "error: easy: 1 engine or search error(s) over 422 games"),
    ("infiniteassault-048", "timeout, neverPlayed: easy: 5 decision(s) over 2000 ms or game(s) past 600 actions; hard (Radiant): affordable in hand on 44 turns and playable on 44 over 24 pass-2 games, never played"),
    ("infiniteassault-072", "neverPlayed: hard (Radiant): affordable in hand on 31 turns and playable on 30 over 24 pass-2 games, never played"),
    ("infiniteassault-123", "timeout: hard: 3 decision(s) over 2000 ms or game(s) past 600 actions"),
    ("infiniteassault-149", "timeout: hard: 4 decision(s) over 2000 ms or game(s) past 600 actions"),
    ("infiniteassault-161", "timeout: hard: 1 decision(s) over 2000 ms or game(s) past 600 actions"),
    ("infiniteassault-211", "neverPlayed: hard (Radiant): affordable in hand on 44 turns and playable on 41 over 24 pass-2 games, never played"),
```

SHADOW_WATCH entries:

```rust
    ("infiniteassault-005", "at risk: easy: pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 0.5; easy (Radiant): pass 1 affordable on 12 turns and playable on 12 over 8 games, played 1 time(s), mean evaluate change 1.3; hard: pass 1 affordable on 4 turns and playable on 4 over 8 games, never played"),
    ("infiniteassault-010", "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 24.9"),
    ("infiniteassault-011", "at risk: hard: pass 1 affordable on 12 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 7.3"),
    ("infiniteassault-012", "at risk: hard: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 14.4"),
    ("infiniteassault-013", "at risk: hard: pass 1 affordable on 14 turns and playable on 14 over 8 games, played 1 time(s), mean evaluate change -1.6"),
    ("infiniteassault-015", "at risk: hard (Radiant): pass 1 affordable on 13 turns and playable on 13 over 8 games, played 1 time(s), mean evaluate change 0.5"),
    ("infiniteassault-017", "at risk: hard (Radiant): pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change 21.3"),
    ("infiniteassault-018", "at risk: hard (Radiant): pass 1 affordable on 8 turns and playable on 5 over 8 games, never played"),
    ("infiniteassault-019", "at risk: hard: pass 1 affordable on 20 turns and playable on 20 over 8 games, never played; hard (Radiant): pass 1 affordable on 12 turns and playable on 12 over 8 games, never played"),
    ("infiniteassault-020", "at risk: hard: pass 1 affordable on 13 turns and playable on 13 over 8 games, never played; hard (Radiant): pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 0.3"),
    ("infiniteassault-021", "at risk: easy: pass 1 affordable on 17 turns and playable on 17 over 8 games, played 1 time(s), mean evaluate change 2.5; hard: pass 1 affordable on 10 turns and playable on 9 over 8 games, played 1 time(s), mean evaluate change 2.5; hard (Radiant): pass 1 affordable on 6 turns and playable on 6 over 8 games, never played"),
    ("infiniteassault-022", "at risk: hard: pass 1 affordable on 7 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 19.5; hard (Radiant): pass 1 affordable on 8 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 25.9"),
    ("infiniteassault-023", "at risk: hard (Radiant): pass 1 affordable on 6 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 17.8"),
    ("infiniteassault-024", "at risk: hard: pass 1 affordable on 9 turns and playable on 9 over 8 games, played 1 time(s), mean evaluate change 26.2"),
    ("infiniteassault-027", "at risk: hard: pass 1 affordable on 6 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 0.8"),
    ("infiniteassault-034", "at risk: hard (Radiant): pass 1 affordable on 15 turns and playable on 15 over 8 games, played 1 time(s), mean evaluate change 12.3"),
    ("infiniteassault-037", "at risk: easy (Radiant): pass 1 affordable on 13 turns and playable on 13 over 8 games, played 1 time(s), mean evaluate change 1.0; hard: pass 1 affordable on 12 turns and playable on 12 over 8 games, played 1 time(s), mean evaluate change 1.5; hard (Radiant): pass 1 affordable on 12 turns and playable on 11 over 8 games, never played"),
    ("infiniteassault-039", "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 3.2"),
    ("infiniteassault-040", "at risk: easy: pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change 2.0; easy (Radiant): pass 1 affordable on 15 turns and playable on 15 over 8 games, played 1 time(s), mean evaluate change 1.5; hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, never played; hard (Radiant): pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 1.5; pass 2 affordable on 35 turns and playable on 31 over 24 games, played 1 time(s), mean evaluate change 1.5"),
    ("infiniteassault-041", "at risk: hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 3.0; hard (Radiant): pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change -2.4"),
    ("infiniteassault-042", "at risk: hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 7.8"),
    ("infiniteassault-043", "at risk: hard: pass 1 affordable on 4 turns and playable on 4 over 8 games, never played; hard (Radiant): pass 1 affordable on 10 turns and playable on 10 over 8 games, never played"),
    ("infiniteassault-045", "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, never played; hard: pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 2.0; hard (Radiant): pass 1 affordable on 5 turns and playable on 5 over 8 games, never played"),
    ("infiniteassault-047", "at risk: hard: pass 1 affordable on 12 turns and playable on 12 over 8 games, never played; hard (Radiant): pass 2 affordable on 45 turns and playable on 43 over 24 games, played 1 time(s), mean evaluate change 1.0"),
    ("infiniteassault-050", "at risk: easy (Radiant): pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change -2.6; hard: pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change -2.1"),
    ("infiniteassault-052", "at risk: hard: pass 1 affordable on 13 turns and playable on 13 over 8 games, played 1 time(s), mean evaluate change 1.0"),
    ("infiniteassault-053", "at risk: hard: pass 1 affordable on 10 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 7.5"),
    ("infiniteassault-056", "at risk: hard: pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 4.3"),
    ("infiniteassault-058", "at risk: easy: pass 1 affordable on 8 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 5.1"),
    ("infiniteassault-061", "at risk: hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s)"),
    ("infiniteassault-062", "at risk: hard (Radiant): pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 0.5"),
    ("infiniteassault-065", "at risk: easy: pass 1 affordable on 5 turns and playable on 5 over 8 games, never played; pass 2 affordable on 502 turns and playable on 503 over 248 games, played 35 time(s), mean evaluate change -1.7, lost the game on 1 play(s); easy (Radiant): pass 1 affordable on 13 turns and playable on 13 over 8 games, never played; hard: pass 1 affordable on 21 turns and playable on 21 over 8 games, played 1 time(s), mean evaluate change -1.9"),
    ("infiniteassault-067", "at risk: hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, never played; hard (Radiant): pass 1 affordable on 14 turns and playable on 14 over 8 games, never played"),
    ("infiniteassault-068", "at risk: easy (Radiant): pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 1.0"),
    ("infiniteassault-071", "at risk: hard: pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 9.5"),
    ("infiniteassault-078", "at risk: hard: pass 1 affordable on 13 turns and playable on 13 over 8 games, played 1 time(s), mean evaluate change 12.9; hard (Radiant): pass 1 affordable on 7 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 12.9"),
    ("infiniteassault-079", "at risk: easy (Radiant): pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 23.6; hard: pass 1 affordable on 15 turns and playable on 10 over 8 games, never played"),
    ("infiniteassault-084", "at risk: easy: pass 1 affordable on 6 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 8.2; hard: pass 1 affordable on 4 turns and playable on 3 over 8 games, never played"),
    ("infiniteassault-085", "at risk: easy: pass 1 affordable on 14 turns and playable on 10 over 8 games, never played; hard (Radiant): pass 1 affordable on 6 turns and playable on 4 over 8 games, never played"),
    ("infiniteassault-086", "at risk: easy: pass 1 affordable on 18 turns and playable on 18 over 8 games, played 1 time(s), mean evaluate change 1.5; hard: pass 1 affordable on 7 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change -0.1"),
    ("infiniteassault-088", "at risk: hard: pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change 3.8; hard (Radiant): pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 19.5"),
    ("infiniteassault-090", "at risk: easy (Radiant): pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 40.9; hard: pass 1 affordable on 7 turns and playable on 7 over 8 games, never played"),
    ("infiniteassault-091", "at risk: hard: pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change 10.1"),
    ("infiniteassault-092", "at risk: hard: pass 1 affordable on 9 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 13.4"),
    ("infiniteassault-093", "at risk: hard: pass 1 affordable on 22 turns and playable on 18 over 8 games, played 1 time(s), mean evaluate change -7.5"),
    ("infiniteassault-095", "at risk: hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 36.4"),
    ("infiniteassault-098", "at risk: hard (Radiant): pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 0.2"),
    ("infiniteassault-099", "at risk: hard: pass 1 affordable on 16 turns and playable on 16 over 8 games, played 1 time(s), mean evaluate change 0.1"),
    ("infiniteassault-100", "at risk: easy (Radiant): pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 10.8"),
    ("infiniteassault-103", "at risk: easy: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 11.6; hard: pass 1 affordable on 9 turns and playable on 9 over 8 games, played 1 time(s), mean evaluate change 21.0; hard (Radiant): pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 22.4"),
    ("infiniteassault-105", "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 7.5; hard: pass 1 affordable on 7 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 7.5"),
    ("infiniteassault-106", "at risk: easy (Radiant): pass 1 affordable on 10 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 10.2"),
    ("infiniteassault-107", "at risk: hard: pass 1 affordable on 14 turns and playable on 14 over 8 games, never played"),
    ("infiniteassault-108", "at risk: easy: pass 1 affordable on 9 turns and playable on 9 over 8 games, played 1 time(s), mean evaluate change 3.6; pass 2 affordable on 364 turns and playable on 364 over 293 games, played 110 time(s), mean evaluate change 1.2, lost the game on 1 play(s); hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, never played"),
    ("infiniteassault-109", "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 7.2; hard: pass 1 affordable on 6 turns and playable on 5 over 8 games, never played"),
    ("infiniteassault-110", "at risk: easy: pass 1 affordable on 19 turns and playable on 19 over 8 games, played 1 time(s), mean evaluate change 2.0; easy (Radiant): pass 1 affordable on 19 turns and playable on 19 over 8 games, never played; hard: pass 1 affordable on 11 turns and playable on 11 over 8 games, never played"),
    ("infiniteassault-111", "at risk: hard: pass 1 affordable on 11 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change 1.0"),
    ("infiniteassault-112", "at risk: hard: pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 4.8; hard (Radiant): pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 6.0"),
    ("infiniteassault-113", "at risk: easy (Radiant): pass 1 affordable on 14 turns and playable on 14 over 8 games, played 1 time(s), mean evaluate change 21.1; hard: pass 1 affordable on 12 turns and playable on 12 over 8 games, never played; hard (Radiant): pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 24.4"),
    ("infiniteassault-114", "at risk: hard: pass 1 affordable on 10 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 11.6"),
    ("infiniteassault-116", "at risk: easy (Radiant): pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 6.1; hard (Radiant): pass 1 affordable on 8 turns and playable on 8 over 8 games, never played"),
    ("infiniteassault-117", "at risk: easy (Radiant): pass 1 affordable on 10 turns and playable on 10 over 8 games, never played; hard: pass 1 affordable on 19 turns and playable on 18 over 8 games, played 1 time(s), mean evaluate change 1.5; hard (Radiant): pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change 1.0"),
    ("infiniteassault-118", "at risk: hard: pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 11.9"),
    ("infiniteassault-119", "at risk: easy (Radiant): pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 43.1; hard: pass 1 affordable on 8 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 15.0; hard (Radiant): pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 26.1"),
    ("infiniteassault-120", "at risk: easy (Radiant): pass 1 affordable on 6 turns and playable on 6 over 8 games, never played; hard: pass 1 affordable on 12 turns and playable on 12 over 8 games, never played"),
    ("infiniteassault-124", "at risk: hard: pass 1 affordable on 17 turns and playable on 17 over 8 games, played 1 time(s), mean evaluate change 0.4; hard (Radiant): pass 1 affordable on 9 turns and playable on 9 over 8 games, never played; pass 2 affordable on 49 turns and playable on 49 over 24 games, played 1 time(s), mean evaluate change -0.5"),
    ("infiniteassault-125", "at risk: easy (Radiant): pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change -0.6; hard (Radiant): pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change -8.1"),
    ("infiniteassault-127", "at risk: easy: pass 1 affordable on 19 turns and playable on 19 over 8 games, never played; easy (Radiant): pass 1 affordable on 17 turns and playable on 17 over 8 games, never played; pass 2 affordable on 71 turns and playable on 71 over 24 games, played 1 time(s), mean evaluate change -2.1; hard: pass 1 affordable on 24 turns and playable on 24 over 8 games, played 1 time(s), mean evaluate change -1.6; hard (Radiant): pass 1 affordable on 7 turns and playable on 7 over 8 games, never played; pass 2 affordable on 48 turns and playable on 48 over 24 games, played 1 time(s), mean evaluate change -2.1"),
    ("infiniteassault-129", "at risk: easy: pass 1 affordable on 9 turns and playable on 5 over 8 games, never played; hard (Radiant): pass 1 affordable on 6 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 11.0"),
    ("infiniteassault-130", "at risk: easy: pass 1 affordable on 13 turns and playable on 13 over 8 games, played 1 time(s), mean evaluate change -1.6; easy (Radiant): pass 1 affordable on 30 turns and playable on 30 over 8 games, never played; hard: pass 1 affordable on 6 turns and playable on 6 over 8 games, never played"),
    ("infiniteassault-132", "at risk: hard: pass 1 affordable on 17 turns and playable on 17 over 8 games, never played; hard (Radiant): pass 1 affordable on 10 turns and playable on 10 over 8 games, never played"),
    ("infiniteassault-134", "at risk: hard: pass 1 affordable on 11 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change 1.5"),
    ("infiniteassault-135", "at risk: hard (Radiant): pass 1 affordable on 7 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 9.5"),
    ("infiniteassault-136", "at risk: easy: pass 1 affordable on 14 turns and playable on 14 over 8 games, played 1 time(s), mean evaluate change 19.3"),
    ("infiniteassault-137", "at risk: easy: pass 1 affordable on 21 turns and playable on 21 over 8 games, never played; easy (Radiant): pass 1 affordable on 17 turns and playable on 17 over 8 games, played 1 time(s), mean evaluate change -2.1; hard: pass 1 affordable on 11 turns and playable on 11 over 8 games, never played; hard (Radiant): pass 1 affordable on 25 turns and playable on 25 over 8 games, never played; pass 2 affordable on 44 turns and playable on 44 over 24 games, played 1 time(s)"),
    ("infiniteassault-138", "at risk: hard: pass 1 affordable on 10 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 18.6; hard (Radiant): pass 1 affordable on 11 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change -16.3"),
    ("infiniteassault-140", "at risk: easy: pass 1 affordable on 26 turns and playable on 25 over 8 games, played 1 time(s), mean evaluate change 2.0; hard (Radiant): pass 1 affordable on 11 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change 1.5"),
    ("infiniteassault-141", "at risk: hard: pass 1 affordable on 21 turns and playable on 20 over 8 games, never played"),
    ("infiniteassault-143", "at risk: easy: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 20.1"),
    ("infiniteassault-144", "at risk: easy: pass 1 affordable on 3 turns and playable on 3 over 8 games, never played; hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 18.9"),
    ("infiniteassault-146", "at risk: hard: pass 1 affordable on 15 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 0.2"),
    ("infiniteassault-148", "at risk: hard (Radiant): pass 1 affordable on 5 turns and playable on 5 over 8 games, never played"),
    ("infiniteassault-150", "at risk: hard: pass 1 affordable on 13 turns and playable on 12 over 8 games, played 1 time(s), mean evaluate change 9.6"),
    ("infiniteassault-154", "at risk: easy: pass 1 affordable on 12 turns and playable on 12 over 8 games, never played; hard: pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 1.5"),
    ("infiniteassault-157", "at risk: easy (Radiant): pass 1 affordable on 7 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 8.7; hard (Radiant): pass 1 affordable on 14 turns and playable on 14 over 8 games, played 1 time(s), mean evaluate change 149.1"),
    ("infiniteassault-158", "at risk: hard (Radiant): pass 1 affordable on 11 turns and playable on 10 over 8 games, never played"),
    ("infiniteassault-160", "at risk: easy: pass 2 affordable on 357 turns and playable on 353 over 236 games, played 64 time(s), mean evaluate change 7.6, lost the game on 1 play(s); hard: pass 1 affordable on 12 turns and playable on 8 over 8 games, never played"),
    ("infiniteassault-164", "at risk: hard: pass 1 affordable on 9 turns and playable on 9 over 8 games, played 1 time(s), mean evaluate change 1.5; hard (Radiant): pass 1 affordable on 21 turns and playable on 20 over 8 games, never played"),
    ("infiniteassault-165", "at risk: easy (Radiant): pass 1 affordable on 16 turns and playable on 16 over 8 games, played 1 time(s), mean evaluate change 0.6; hard: pass 1 affordable on 21 turns and playable on 19 over 8 games, played 1 time(s), mean evaluate change 2.0; hard (Radiant): pass 1 affordable on 11 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change 1.5"),
    ("infiniteassault-166", "at risk: hard (Radiant): pass 1 affordable on 4 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 36.7"),
    ("infiniteassault-167", "at risk: hard: pass 1 affordable on 4 turns and playable on 4 over 8 games, never played"),
    ("infiniteassault-168", "at risk: easy: pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change -1.9; hard: pass 1 affordable on 11 turns and playable on 11 over 8 games, never played; hard (Radiant): pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 4.5"),
    ("infiniteassault-169", "at risk: easy: pass 1 affordable on 6 turns and playable on 5 over 8 games, never played"),
    ("infiniteassault-171", "at risk: hard: pass 1 affordable on 8 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 5.0"),
    ("infiniteassault-172", "at risk: hard: pass 1 affordable on 12 turns and playable on 12 over 8 games, never played; hard (Radiant): pass 1 affordable on 6 turns and playable on 6 over 8 games, never played"),
    ("infiniteassault-174", "at risk: hard: pass 1 affordable on 7 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 0.8"),
    ("infiniteassault-175", "at risk: hard (Radiant): pass 1 affordable on 11 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 5.5"),
    ("infiniteassault-176", "at risk: hard: pass 1 affordable on 8 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 14.3"),
    ("infiniteassault-177", "at risk: hard (Radiant): pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 1.4"),
    ("infiniteassault-178", "at risk: hard (Radiant): pass 1 affordable on 4 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 20.1"),
    ("infiniteassault-179", "at risk: hard: pass 1 affordable on 6 turns and playable on 6 over 8 games, never played"),
    ("infiniteassault-182", "at risk: hard: pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 7.0; hard (Radiant): pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 30.0"),
    ("infiniteassault-183", "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 1.5; hard: pass 1 affordable on 15 turns and playable on 15 over 8 games, never played"),
    ("infiniteassault-184", "at risk: hard (Radiant): pass 1 affordable on 11 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change -1.8"),
    ("infiniteassault-185", "at risk: hard: pass 1 affordable on 8 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 13.6"),
    ("infiniteassault-186", "at risk: hard: pass 1 affordable on 15 turns and playable on 15 over 8 games, never played"),
    ("infiniteassault-187", "at risk: easy: pass 1 affordable on 10 turns and playable on 10 over 8 games, never played; hard (Radiant): pass 1 affordable on 10 turns and playable on 9 over 8 games, never played"),
    ("infiniteassault-188", "at risk: easy: pass 1 affordable on 21 turns and playable on 21 over 8 games, played 1 time(s); easy (Radiant): pass 1 affordable on 21 turns and playable on 16 over 8 games, played 1 time(s), mean evaluate change -6.4; hard (Radiant): pass 1 affordable on 3 turns and playable on 3 over 8 games, never played"),
    ("infiniteassault-189", "at risk: hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 2.5; hard (Radiant): pass 2 affordable on 19 turns and playable on 17 over 24 games, played 1 time(s), mean evaluate change 2.0"),
    ("infiniteassault-191", "at risk: hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 23.5"),
    ("infiniteassault-192", "at risk: easy: pass 1 affordable on 12 turns and playable on 12 over 8 games, never played; easy (Radiant): pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 3.0; hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, never played; hard (Radiant): pass 1 affordable on 3 turns and playable on 3 over 8 games, never played; pass 2 affordable on 29 turns and playable on 29 over 24 games, played 1 time(s), mean evaluate change 1.4"),
    ("infiniteassault-194", "at risk: easy: pass 1 affordable on 4 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 1.7; easy (Radiant): pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 2.7; hard: pass 1 affordable on 6 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 1.7"),
    ("infiniteassault-195", "at risk: hard: pass 1 affordable on 10 turns and playable on 9 over 8 games, played 1 time(s), mean evaluate change 11.6"),
    ("infiniteassault-196", "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 25.6"),
    ("infiniteassault-199", "at risk: hard: pass 1 affordable on 10 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 4.4"),
    ("infiniteassault-202", "at risk: easy (Radiant): pass 1 affordable on 4 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 6.2; hard: pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 3.5"),
    ("infiniteassault-203", "at risk: easy: pass 1 affordable on 7 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 1.5"),
    ("infiniteassault-207", "at risk: hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, never played"),
    ("infiniteassault-213", "at risk: easy (Radiant): pass 1 affordable on 10 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change -2.7; hard: pass 1 affordable on 7 turns and playable on 6 over 8 games, never played; hard (Radiant): pass 1 affordable on 18 turns and playable on 14 over 8 games, never played; pass 2 affordable on 35 turns and playable on 22 over 24 games, played 1 time(s), mean evaluate change -2.7"),
    ("infiniteassault-214", "at risk: hard (Radiant): pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 10.5"),
    ("infiniteassault-218", "at risk: easy (Radiant): pass 1 affordable on 13 turns and playable on 13 over 8 games, played 1 time(s), mean evaluate change 1.0; hard (Radiant): pass 1 affordable on 12 turns and playable on 12 over 8 games, played 1 time(s), mean evaluate change 1.0"),
    ("infiniteassault-220", "at risk: easy: pass 1 affordable on 7 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 15.4; easy (Radiant): pass 1 affordable on 14 turns and playable on 14 over 8 games, played 1 time(s), mean evaluate change 19.8"),
    ("infiniteassault-223", "at risk: hard: pass 1 affordable on 8 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 16.0"),
    ("infiniteassault-224", "at risk: hard: pass 1 affordable on 12 turns and playable on 12 over 8 games, played 1 time(s), mean evaluate change 59.6; hard (Radiant): pass 1 affordable on 15 turns and playable on 15 over 8 games, played 1 time(s), mean evaluate change 47.8"),
    ("infiniteassault-226", "at risk: hard (Radiant): pass 1 affordable on 12 turns and playable on 12 over 8 games, never played"),
    ("infiniteassault-228", "at risk: easy: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 2.0; hard: pass 1 affordable on 14 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change 2.0; hard (Radiant): pass 1 affordable on 14 turns and playable on 13 over 8 games, never played"),
    ("infiniteassault-234", "at risk: hard: pass 1 affordable on 13 turns and playable on 13 over 8 games, played 1 time(s), mean evaluate change 37.5; hard (Radiant): pass 1 affordable on 15 turns and playable on 15 over 8 games, played 1 time(s), mean evaluate change 25.4"),
    ("infiniteassault-236", "at risk: hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 10.9"),
    ("infiniteassault-237", "at risk: hard: pass 1 affordable on 5 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 25.3"),
    ("infiniteassault-238", "at risk: easy: pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 2.0; hard: pass 1 affordable on 14 turns and playable on 14 over 8 games, played 1 time(s), mean evaluate change 2.0; hard (Radiant): pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change -0.1"),
    ("infiniteassault-240", "at risk: easy: pass 1 affordable on 14 turns and playable on 14 over 8 games, never played; easy (Radiant): pass 1 affordable on 15 turns and playable on 15 over 8 games, played 1 time(s), mean evaluate change 1.5; pass 2 affordable on 52 turns and playable on 48 over 24 games, played 1 time(s), mean evaluate change 1.5; hard: pass 1 affordable on 7 turns and playable on 7 over 8 games, never played; hard (Radiant): pass 1 affordable on 9 turns and playable on 8 over 8 games, never played"),
```

Cleared ids:

```rust
    "infiniteassault-001",
    "infiniteassault-002",
    "infiniteassault-003",
    "infiniteassault-004",
    "infiniteassault-006",
    "infiniteassault-007",
    "infiniteassault-008",
    "infiniteassault-009",
    "infiniteassault-014",
    "infiniteassault-016",
    "infiniteassault-025",
    "infiniteassault-026",
    "infiniteassault-028",
    "infiniteassault-030",
    "infiniteassault-031",
    "infiniteassault-032",
    "infiniteassault-033",
    "infiniteassault-036",
    "infiniteassault-038",
    "infiniteassault-044",
    "infiniteassault-046",
    "infiniteassault-049",
    "infiniteassault-051",
    "infiniteassault-054",
    "infiniteassault-055",
    "infiniteassault-057",
    "infiniteassault-059",
    "infiniteassault-060",
    "infiniteassault-063",
    "infiniteassault-064",
    "infiniteassault-066",
    "infiniteassault-069",
    "infiniteassault-070",
    "infiniteassault-073",
    "infiniteassault-074",
    "infiniteassault-075",
    "infiniteassault-076",
    "infiniteassault-077",
    "infiniteassault-080",
    "infiniteassault-081",
    "infiniteassault-082",
    "infiniteassault-083",
    "infiniteassault-087",
    "infiniteassault-089",
    "infiniteassault-094",
    "infiniteassault-096",
    "infiniteassault-097",
    "infiniteassault-101",
    "infiniteassault-102",
    "infiniteassault-104",
    "infiniteassault-115",
    "infiniteassault-121",
    "infiniteassault-122",
    "infiniteassault-126",
    "infiniteassault-128",
    "infiniteassault-131",
    "infiniteassault-133",
    "infiniteassault-139",
    "infiniteassault-142",
    "infiniteassault-145",
    "infiniteassault-147",
    "infiniteassault-151",
    "infiniteassault-152",
    "infiniteassault-153",
    "infiniteassault-155",
    "infiniteassault-156",
    "infiniteassault-159",
    "infiniteassault-162",
    "infiniteassault-163",
    "infiniteassault-170",
    "infiniteassault-173",
    "infiniteassault-180",
    "infiniteassault-181",
    "infiniteassault-190",
    "infiniteassault-193",
    "infiniteassault-197",
    "infiniteassault-198",
    "infiniteassault-200",
    "infiniteassault-201",
    "infiniteassault-204",
    "infiniteassault-205",
    "infiniteassault-206",
    "infiniteassault-208",
    "infiniteassault-209",
    "infiniteassault-210",
    "infiniteassault-212",
    "infiniteassault-215",
    "infiniteassault-216",
    "infiniteassault-217",
    "infiniteassault-219",
    "infiniteassault-221",
    "infiniteassault-222",
    "infiniteassault-225",
    "infiniteassault-227",
    "infiniteassault-229",
    "infiniteassault-230",
    "infiniteassault-231",
    "infiniteassault-232",
    "infiniteassault-233",
    "infiniteassault-235",
    "infiniteassault-239",
```

## What the AI misplayed

One row per card the sweep banned. Each `neverPlayed` card has a row in
`crates/ai/tests/ai/assault.rs`'s clear-spot test (`assault_the_ai_plays_each_swept_card_in_its_clear_spot`):
the AI plays it, at `AI_GATE_BUDGET` and `AI_BUDGET` on two seeds, where its text says to.

| id | name | flags | tier/face | test | verdict |
|---|---|---|---|---|---|
| infiniteassault-029 | False Positive | timeout | easy/base | `heavy_steps::r4595003_…` | fixed by R4595003: a Drone death loop ran to the engine's cap (2,002 events) inside one search step, which the node counter saw as one node; a step now pays a node per hundred events past its first thousand (`AI_SEARCH.free_events`), so only a loop toward the cap pays |
| infiniteassault-035 | Fata Morgana | error | easy/base | `decide::r4595001_…` | error fixed by R4595001: a chain of casts that each announced a Zephrys spent the whole budget before the beam closed a line, and the decision fell back (an error); it now takes its best one-step candidate |
| infiniteassault-048 | Ego Death | timeout, neverPlayed | easy/base (timeout), hard/radiant (neverPlayed) | `scorer_casts::r4595004_…`, clear-spot row | timeout fixed by R4595004: revealing a face-down Zephrys was not charged as the expensive play it is (a 10.9 s decision, 1.9 s after); neverPlayed: piloted; passed over (hard plays it 23 times on its base face and holds the Radiant face for better plays) |
| infiniteassault-072 | Overclock to Infinity | neverPlayed | hard/radiant | `discount_lines::r4595002_…`, clear-spot row | fixed by R4595002: the beam cut the line that played on from Overclock's discount at its width before the discount paid off; a line that played a listed discount card (`AI_SEARCH.discount_def_ids`: Overclock alone) and cheapened the hand now keeps a place past the width (`AI_SEARCH.discount_lines`) |
| infiniteassault-123 | Cost Injection | timeout | hard/base | none | timeout was contention |
| infiniteassault-149 | Encrypt | timeout | hard/base | none | timeout was contention (see Re-measure: one decision sits at the limit) |
| infiniteassault-161 | Defense in Depth | timeout | hard/base | none | timeout was contention |
| infiniteassault-211 | Grey Goo | neverPlayed | hard/radiant | clear-spot row | piloted; passed over (played in every clear-spot row; hard plays its base face 11 times and passes the Radiant face over) |

Ego Death's and Grey Goo's Radiant faces, which hard never plays in the sweep's games, are for the card
owner to tune or accept (#4595, Open question 1). No card was redesigned, and no card's text changed.

## Re-measure

The eight banned cards, swept again on the fixed AI (`jackioh-fixed`, this branch): pass 1 on 4 lanes, then
pass 2 on 3 lanes from the original pass-1 lines plus the new ones, so each card stays at risk and its pass 2
runs.

- Fata Morgana (#35), Overclock to Infinity (#72), False Positive (#29), Cost Injection (#123) and Defense in
  Depth (#161) are no longer banned. Overclock is now played on every face and tier (5 to 15 times a face in pass 2).
- Ego Death (#48) and Grey Goo (#211) are banned only for `neverPlayed` on hard's Radiant face: piloted;
  passed over (above). Ego Death's easy timeouts are gone (0 re-timed in pass 2, against 5 before).
- Encrypt (#149) read one timeout on hard's base face: the same decision as the original sweep's (game
  `sweep:hard:infiniteassault-149:7`, turn 13, 1,140 nodes, about 12 s raw). Run alone three times, R977000's
  re-timer gave it 1,940 ms (base AI), 2,242 ms and 1,756 ms (fixed AI) against `AI_SWEEP.decision_ms` 2,000,
  so it sits on the limit on both builds, and two of three lone runs read `"timeouts":0` on all four lines.
  The fixes did not cause it, and by Step 7's rule it is contention.

The re-measure above ran on the first reading of R4595002 and R4595003. The fork screen (R4397000,
`arena --a self --b bin:<main> --set improve-dev --forks`) forked shipped decks' games under it: 20 forks
over 200 games, `forkEdge` -0.031 (interval -0.127 to 0.064), inconclusive. Both rulings were narrowed
(their Scope paragraphs): R4595002 reads only `AI_SEARCH.discount_def_ids` (Overclock), and R4595003 charges
only events past the first thousand (`AI_SEARCH.free_events`). On that build, against a control built from the
same `main`:

- the fork screen read `"forks":0` over 200 games;
- pass 1 of False Positive (#29) and Overclock to Infinity (#72) read clean on all eight lines, at
  `"timeouts":0` and `"errors":0`, with Overclock played on every face and tier (1 to 4 times a line);
- `cargo jackioh gate` at smoke size passed: ai-vs-random 19/20, ai-vs-greedy 19/20, hard-vs-easy 18/20,
  no search panics.

Run 2026-11-26 (UTC): pass 1 over 8 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 8 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), each card also on its Radiant face (`sweep:<tier>:<id>:radiant:<n>`, `sweep2:<tier>:<id>:radiant:<n>`), budget AI_GATE_BUDGET {"nodes":1200,"lethalNodes":300,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}; 3 banned, 5 watched, 0 unswept, 0 suspect line(s).

### Pass 1 flags (a `neverPlayed`, `selfHarm` or `selfKill` here only puts a card at risk)

| id | name | tier | face | flags | drawn/games | affordable turns | playable turns | plays | errors | timeouts | raw timeouts | mean eval delta |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| infiniteassault-048 | Ego Death | hard | base | neverPlayed | 2/8 | 6 | 6 | 0 | 0 | 0 | 2 | n/a |
| infiniteassault-123 | Cost Injection | easy | base | neverPlayed | 3/8 | 7 | 7 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-123 | Cost Injection | easy | radiant | neverPlayed | 5/8 | 24 | 24 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-123 | Cost Injection | hard | base | neverPlayed | 2/8 | 5 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-123 | Cost Injection | hard | radiant | neverPlayed | 3/8 | 7 | 7 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-149 | Encrypt | hard | base | timeout | 3/8 | 5 | 5 | 2 | 0 | 1 | 5 | 3.7 |
| infiniteassault-211 | Grey Goo | hard | base | neverPlayed | 8/8 | 29 | 22 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-211 | Grey Goo | hard | radiant | neverPlayed | 6/8 | 24 | 18 | 0 | 0 | 0 | 0 | n/a |

### At risk (264): pass 1 at half strength, SHADOW_BAN and SHADOW_WATCH

classic-006 Cloaked Toe Cracker, classic-015 Nose Hunter, classic-018 Glitch in the System, classic-026 Rapid Draw, classic-037 Last Hurrah, classic-038 Jackiestan Auctioneer, classic-042 Transmutable Toxins, classic-043 Plague Nuke, classic-044 Back from the GY, classic-050 Voidwalker, classic-055 Book of Wildfire, classic-062 Living Bomb, classic-065 Ace in the Hole, classic-066 EU Striker, classic-070 Book of Plague, classic-074 Corpse Plantation, classic-078 Mutate Spell, classic-080 BOOM! Big Max, classic-089 Paul Allen's Ghost, classicplus-003 Second Amendment Snake, classicplus-009 Silence, classicplus-014 Forever&, classicplus-017 Conjure Rush Token++, classicplus-022 Blood Moon, classicplus-029 Portal to the Past, classicplus-030 Felinor Fuser, classicplus-031 Fusion Lab, classicplus-033 Ivory Tower, classicplus-034 Memory Leak, classicplus-036 Conjure Bones, classicplus-041 KY's Constant, classicplus-042 KY's Test, classicplus-043 AI Slop, classicplus-044 Simplicity Audit, classicplus-045 Complexity Audit, classicplus-048 Jlockeed's Lobbyist, classicplus-065 Two Grapes, classicplus-066 Vine of Grapes, classicplus-073 Call to Chaos (Classic+ Edition), classicplus-078 Claude's Datacenter, core-001 Big D-fender, core-005 Stockpile, core-006 Mana Well, core-007 Jewelosco Scarab, core-016 Hit Job, core-017 Flood, core-022 Carnivorous Cube, core-026 Glowy Jelly Bean, core-034 Collateral Damage, core-040 Echoes of the Forgotten, core-042 Selective Breeding, core-048 5pek Controller, core-057 Conjure KY, core-058 Rush Token Farm, core-059 Unbiased Immigration, core-065 Masochism Mask, core-069 Call to Arms, core-072 Reminisce, core-079 Twinspell, core-083 Transmogulate, core-086 "Miss" Mrow, core-087 Pocket Chaos, core-090 Super-Virus Injection, core-092 Felinor Fiender, core-093 Combo-Index, core-094 Genn's Greed, core-099 Craft a Card, infiniteassault-029 False Positive, infiniteassault-035 Fata Morgana, infiniteassault-048 Ego Death, infiniteassault-072 Overclock to Infinity, infiniteassault-123 Cost Injection, infiniteassault-149 Encrypt, infiniteassault-161 Defense in Depth, infiniteassault-211 Grey Goo, meditative-011 Double Header, meditative-022 Mind Games, meditative-026 Alternate Fate, meditative-053 Prestige, meditative-055 Dragon Fruit, meditative-060 Eschews, meditative-069 The Maestro, meditative-075 Ever Growing Tree, meditative-098 Showdown, meditative-099 Paranoia, meditativeplus-002 NA Tech, meditativeplus-003 Wondrous Wand, meditativeplus-004 Jlockeed's Supply Drop, meditativeplus-005 Spirit Lash, meditativeplus-013 Living Health Potion, meditativeplus-014 Potion Pouch, meditativeplus-020 KY's Competition, meditativeplus-022 Mountaintop Garden, meditativeplus-023 Enormous Elderberry, stinkyscape-002 Absolute Zero, stinkyscape-004 Olivian Expanse, stinkyscape-007 The Greatest Heist, stinkyscape-008 Conjure Rosemary Triscuit, stinkyscape-009 Ticketed, stinkyscape-010 Otherworldly Endurance, stinkyscape-011 Otherworldly Magic, stinkyscape-017 Imprisoned Ship Smith, stinkyscape-019 Imprisoned Cosmality, stinkyscape-021 Moonberry, stinkyscape-022 Gatinsir, stinkyscape-024 Warp Disruptor II, stinkyscape-032 Drone Core VI, stinkyscape-036 Drone Core X, stinkyscape-039 The Narcor Project, stinkyscape-040 Poison Capsule, stinkyscape-046 Circadian Halfcycler, stinkyscape-047 Circadian Fullcycler, stinkyscape-049 Spacemine, stinkyscape-052 Cannon v3, stinkyscape-055 Repeater v4, stinkyscape-058 Beam v5, stinkyscape-059 Repeater v5, stinkyscape-061 Flak v5, stinkyscape-063 Miner v2, stinkyscape-064 Miner v3, stinkyscape-065 Miner v4, stinkyscape-066 Miner v5, stinkyscape-071 A'Zoth, stinkyscape-074 Paragon-X, stinkyscape-076 Honey Badger, stinkyscape-080 Galaxy, stinkyscape-082 Phalanx, stinkyscape-083 Halo, stinkyscape-084 Power Cell, stinkyscape-085 Prototype-X1, stinkyscape-086 Prototype-X2, stinkyscape-087 Siva 1, stinkyscape-088 Siva 2, stinkyscape-089 Spice, stinkyscape-093 Patcher Nanobots, stinkyscape-095 Faction: Vanguard Coalition, stinkyscape-096 Ocean Tsunami, stinkyscape-104 Reflective Plating, stinkyscape-109 xxFreshJack, stinkyscape-114 Arctic Citadel, stinkyscape-115 Scornboi, stinkyscape-117 Core System, stinkyscape-118 Secure System, stinkyscape-119 Unsecure System, stinkyscape-120 Wild System, stinkyscape-121 System Schematic, stinkyscape-124 Jlockeed's Shipbuilding Grant, stinkyscape-125 Jlockeed's Spacerace Propaganda, stinkyscape-126 Jlockeed's Permanent Procurement Lobbying, stinkyscape-127 Jlockeed's Siege Tank, stinkyscape-128 Jlockeed's Yamato Cannon, stinkyscape-130 Jlockeed's Starship Retrofitting Program, stinkyscape-131 Spacerace Stimulus, stinkyscape-132 Defensive Crystal, stinkyscape-133 Ultra Capacitor, stinkyscape-134 Felinor Containment Chamber, stinkyscape-141 Fruit Launcher, stinkyscape-144 Prepare For Liftoff, stinkyscape-145 Fortify Flesh, stinkyscape-150 Call to Chaos (Stinky Edition), stinkyscape-151 Call to Chaos (Scape Edition), stinkyscape-152 Sanctum, stinkyscape-155 Comet Crash, stinkyscape-156 Catalyst Comet, stinkyscape-157 The Gauntlet, stinkyscape-158 Stratos Swarm, stinkyscape-159 Faction: ETHICS, stinkyscape-161 Faction: Pegasus, stinkyscape-162 Faction: Star-Jumper Core, stinkyscape-163 Swear Loyalities, stinkyscape-164 The Syndicate, stinkyscape-165 The Trade Union, stinkyscape-166 Scanning Probe Mk1, stinkyscape-167 Scanning Probe Mk2, stinkyscape-171 Large Asteroid, stinkyscape-172 Huge Asteroid, stinkyscape-173 Abandoned Spice Tower, stinkyscape-174 Aerelret, stinkyscape-175 Frost Module, stinkyscapeplus-001 Starfruit, stinkyscapeplus-004 Profe Yates, stinkyscapeplus-005 Dingleberry Droat, stinkyscapeplus-008 Drone Assembler v3, stinkyscapeplus-009 Drone Assembler v4, stinkyscapeplus-013 Shield Repairer, stinkyscapeplus-015 Hull Repairer mk2, stinkyscapeplus-023 GIGA-ify, stinkyscapeplus-024 Forced Propagation, stinkyscapeplus-026 Jlockeed’s Forever War, stinkyscapeplus-030 Yeah I’m a War Hero, stinkyscapeplus-032 Z-Patch, stinkyscapeplus-033 True Conservative, stinkyscapeplus-038 Arc, stinkyscapeplus-040 Mender, stinkyscapeplus-041 Mend Projector, stinkyscapeplus-043 Segment, stinkyscapeplus-046 Phase Fabric, stinkyscapeplus-049 Metaglass, stinkyscapeplus-050 Graphite, stinkyscapeplus-052 Airblast Drill, stinkyscapeplus-053 Laser Drill, stinkyscapeplus-054 Pneumatic Drill, stinkyscapeplus-056 Impact Reactor, stinkyscapeplus-057 Phase Weaver, stinkyscapeplus-060 Radi, stinkyscapeplus-063 Overdrive Dome, stinkyscapeplus-068 Shadiris Visions, stinkyscapeplus-069 Exotic Jellybean, stinkyscapeplus-072 Foreshadow, stinkyscapeplus-073 Ripple, stinkyscapeplus-077 Black Hole, stinkyscapeplus-079 A Glimpse into the Endless Sea, stinkyscapeplus-085 Body Snatch, stinkyscapeplus-087 Frontier Flora, stinkyscapeplus-098 Doomed Ship, stinkyscapeplus-103 Transcend, stinkyscapeplus-106 Spectre, stinkyscapeplus-109 Lancer, stinkyscapeplus-111 Mace, stinkyscapeplus-113 Scepter, stinkyscapeplus-119 Eclipse, stinkyscapeplus-123 Oct, stinkyscapeplus-125 Omura, stinkyscapeplus-126 Vela, stinkyscapeplus-129 Multiplicative Reconstructor, stinkyscapeplus-130 Exponential Reconstructor, stinkyscapeplus-131 Tetrative Reconstructor, stinkyscapeplus-133 Great Paperclip Theory III, stinkyscapeplus-136 Crawler Assembly Line, stinkyscapeplus-138 Delta Definition, stinkyscapeplus-140 Automa Reintegration Plan, stinkyscapeplus-142 Call to Chaos (Mindustrial-Swarm Edition), stinkyscapeplus-143 Fire Beam v3, stinkyscapeplus-144 Fire Beam v4, stinkyscapeplus-145 Fire Beam v5, stinkyscapeplus-146 Scourge Mortar v3, stinkyscapeplus-147 Scourge Mortar v4, stinkyscapeplus-148 Scourge Mortar v5, stinkyscapeplus-150 Scorching Rounds, stinkyscapeplus-151 Volcanic Rounds, stinkyscapeplus-153 Noxious Rounds, stinkyscapeplus-155 The Far Vagabond, stinkyscapeplus-156 Horizon Breaker, stinkyscapeplus-160 Icarus, stinkyscapeplus-165 Shadiris, stinkyscapeplus-166 Mindustrial Revolution, traininggrounds-007 Chosen by the Moon, traininggrounds-012 Fortify, traininggrounds-013 Explosive Runes, traininggrounds-016 Celestial Caller, traininggrounds-017 Flaming Felinor, traininggrounds-019 Extra Rations, traininggrounds-020 Recruitment Posters, traininggrounds-031 Level Up!, traininggrounds-035 Corpse Recycler, traininggrounds-036 Training Dummy, traininggrounds-037 Seance, traininggrounds-038 Tutorial Master, traininggrounds-046 Legacy: Hit Job, traininggrounds-049 Legacy: Reoccurring Dream, traininggrounds-050 Legacy: Glowy Jelly Bean, traininggrounds-053 Legacy: Prejudiced Postdoc, traininggrounds-055 Legacy: Reminisce, traininggrounds-060 Legacy: Rush Token Farm

### Pass 2: every at-risk card over every game it was dealt in, forced or filler

| id | name | tier | face | flags | drawn/games | affordable turns | playable turns | plays | errors | timeouts | raw timeouts | mean eval delta |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| classic-006 | Cloaked Toe Cracker | easy | base | cleared | 13/25 | 18 | 18 | 10 | 0 | 0 | 0 | 8.9 |
| classic-006 | Cloaked Toe Cracker | hard | base | cleared | 10/23 | 18 | 17 | 8 | 0 | 0 | 0 | 11.6 |
| classic-015 | Nose Hunter | easy | base | cleared | 9/20 | 15 | 15 | 9 | 0 | 0 | 0 | 4.7 |
| classic-015 | Nose Hunter | hard | base | cleared | 11/26 | 19 | 15 | 6 | 0 | 0 | 0 | 5.3 |
| classic-018 | Glitch in the System | easy | base | cleared | 1/4 | 2 | 2 | 1 | 0 | 0 | 0 | 29.1 |
| classic-018 | Glitch in the System | hard | base | cleared | 13/18 | 29 | 29 | 7 | 0 | 0 | 0 | 12.4 |
| classic-026 | Rapid Draw | easy | base | cleared | 4/9 | 7 | 7 | 3 | 0 | 0 | 0 | 0.5 |
| classic-026 | Rapid Draw | hard | base | cleared | 7/17 | 6 | 6 | 6 | 0 | 0 | 0 | 1.0 |
| classic-037 | Last Hurrah | easy | base | cleared | 7/14 | 6 | 6 | 7 | 0 | 0 | 0 | 10.5 |
| classic-037 | Last Hurrah | hard | base | cleared | 6/7 | 5 | 5 | 4 | 0 | 0 | 0 | 13.4 |
| classic-038 | Jackiestan Auctioneer | easy | base | cleared | 7/10 | 17 | 17 | 4 | 0 | 0 | 0 | 6.9 |
| classic-038 | Jackiestan Auctioneer | hard | base | cleared | 3/7 | 7 | 7 | 3 | 0 | 0 | 0 | 5.6 |
| classic-042 | Transmutable Toxins | easy | base | cleared | 6/9 | 10 | 10 | 5 | 0 | 0 | 0 | 3.4 |
| classic-042 | Transmutable Toxins | hard | base | cleared | 9/23 | 12 | 12 | 7 | 0 | 0 | 0 | 2.3 |
| classic-043 | Plague Nuke | easy | base | cleared | 7/10 | 16 | 16 | 1 | 0 | 0 | 0 | 13.3 |
| classic-043 | Plague Nuke | hard | base | cleared | 10/24 | 25 | 25 | 6 | 0 | 0 | 0 | 6.2 |
| classic-044 | Back from the GY | easy | base | cleared | 9/12 | 13 | 13 | 8 | 0 | 0 | 0 | -2.2 |
| classic-044 | Back from the GY | hard | base | cleared | 11/16 | 35 | 35 | 5 | 0 | 0 | 0 | -2.3 |
| classic-050 | Voidwalker | easy | base | cleared | 16/29 | 26 | 26 | 14 | 0 | 0 | 0 | 11.2 |
| classic-050 | Voidwalker | hard | base | cleared | 12/25 | 22 | 20 | 8 | 0 | 0 | 0 | 10.9 |
| classic-055 | Book of Wildfire | easy | base | cleared | 7/12 | 4 | 4 | 6 | 0 | 0 | 0 | 5.0 |
| classic-055 | Book of Wildfire | hard | base | cleared | 3/13 | 2 | 2 | 0 | 0 | 0 | 0 | n/a |
| classic-062 | Living Bomb | easy | base | cleared | 11/17 | 25 | 25 | 4 | 0 | 0 | 0 | 2.8 |
| classic-062 | Living Bomb | hard | base | cleared | 7/20 | 15 | 14 | 3 | 0 | 0 | 0 | 2.0 |
| classic-065 | Ace in the Hole | easy | base | cleared | 6/9 | 25 | 25 | 4 | 0 | 0 | 0 | 3.4 |
| classic-065 | Ace in the Hole | hard | base | cleared | 10/17 | 24 | 24 | 7 | 0 | 0 | 0 | 3.0 |
| classic-066 | EU Striker | easy | base | cleared | 17/30 | 26 | 23 | 9 | 0 | 0 | 0 | 16.1 |
| classic-066 | EU Striker | hard | base | cleared | 9/19 | 22 | 13 | 1 | 0 | 0 | 0 | 13.3 |
| classic-070 | Book of Plague | easy | base | cleared | 5/18 | 8 | 8 | 3 | 0 | 0 | 0 | 1.8 |
| classic-070 | Book of Plague | hard | base | cleared | 14/26 | 20 | 20 | 12 | 0 | 0 | 0 | 3.0 |
| classic-074 | Corpse Plantation | easy | base | cleared | 7/16 | 19 | 19 | 7 | 0 | 0 | 0 | 1.1 |
| classic-074 | Corpse Plantation | hard | base | cleared | 17/24 | 69 | 68 | 6 | 0 | 0 | 0 | 1.2 |
| classic-078 | Mutate Spell | easy | base | cleared | 8/19 | 31 | 31 | 6 | 0 | 0 | 0 | 0.5 |
| classic-078 | Mutate Spell | hard | base | cleared | 7/16 | 16 | 16 | 5 | 0 | 0 | 0 | 0.4 |
| classic-080 | BOOM! Big Max | easy | base | cleared | 12/16 | 29 | 14 | 9 | 0 | 0 | 0 | 11.2 |
| classic-080 | BOOM! Big Max | hard | base | cleared | 15/37 | 34 | 19 | 9 | 0 | 0 | 0 | 12.1 |
| classic-089 | Paul Allen's Ghost | easy | base | cleared | 9/18 | 11 | 10 | 7 | 0 | 0 | 0 | 16.7 |
| classic-089 | Paul Allen's Ghost | hard | base | cleared | 13/23 | 25 | 13 | 10 | 0 | 0 | 0 | 16.1 |
| classicplus-003 | Second Amendment Snake | easy | base | cleared | 13/25 | 23 | 20 | 10 | 0 | 0 | 0 | 6.7 |
| classicplus-003 | Second Amendment Snake | hard | base | cleared | 14/29 | 28 | 24 | 8 | 0 | 0 | 0 | 7.4 |
| classicplus-009 | Silence | easy | base | cleared | 6/9 | 13 | 13 | 5 | 0 | 0 | 0 | 2.4 |
| classicplus-009 | Silence | hard | base | cleared | 8/13 | 19 | 19 | 5 | 0 | 0 | 0 | -1.4 |
| classicplus-014 | Forever& | easy | base | cleared | 3/9 | 5 | 5 | 0 | 0 | 0 | 0 | n/a |
| classicplus-014 | Forever& | hard | base | cleared | 2/13 | 6 | 6 | 2 | 0 | 0 | 0 | -1.3 |
| classicplus-017 | Conjure Rush Token++ | easy | base | cleared | 5/7 | 6 | 6 | 3 | 0 | 0 | 0 | 13.3 |
| classicplus-017 | Conjure Rush Token++ | hard | base | cleared | 6/9 | 15 | 15 | 2 | 0 | 0 | 0 | 21.7 |
| classicplus-022 | Blood Moon | easy | base | cleared | 6/12 | 20 | 20 | 5 | 0 | 0 | 0 | 2.7 |
| classicplus-022 | Blood Moon | hard | base | cleared | 9/16 | 33 | 28 | 6 | 0 | 0 | 0 | 1.2 |
| classicplus-029 | Portal to the Past | easy | base | cleared | 4/6 | 15 | 15 | 3 | 0 | 0 | 0 | -1.9 |
| classicplus-029 | Portal to the Past | hard | base | cleared | 11/15 | 38 | 38 | 2 | 0 | 0 | 0 | -1.9 |
| classicplus-030 | Felinor Fuser | easy | base | cleared | 9/13 | 7 | 3 | 3 | 0 | 0 | 0 | 5.4 |
| classicplus-030 | Felinor Fuser | hard | base | cleared | 13/25 | 25 | 12 | 8 | 0 | 0 | 0 | 6.2 |
| classicplus-031 | Fusion Lab | easy | base | cleared | 2/8 | 13 | 13 | 1 | 0 | 0 | 0 | 1.5 |
| classicplus-031 | Fusion Lab | hard | base | cleared | 9/22 | 25 | 25 | 4 | 0 | 0 | 0 | 1.4 |
| classicplus-033 | Ivory Tower | easy | base | cleared | 4/8 | 6 | 5 | 1 | 0 | 0 | 0 | 2.0 |
| classicplus-033 | Ivory Tower | hard | base | cleared | 8/13 | 14 | 14 | 1 | 0 | 0 | 0 | 2.1 |
| classicplus-034 | Memory Leak | easy | base | cleared | 9/13 | 26 | 26 | 7 | 0 | 0 | 0 | -0.6 |
| classicplus-034 | Memory Leak | hard | base | cleared | 5/12 | 6 | 5 | 3 | 0 | 0 | 0 | 1.1 |
| classicplus-036 | Conjure Bones | easy | base | cleared | 7/12 | 18 | 18 | 3 | 0 | 0 | 0 | -1.7 |
| classicplus-036 | Conjure Bones | hard | base | cleared | 10/14 | 42 | 42 | 1 | 0 | 0 | 0 | 1.5 |
| classicplus-041 | KY's Constant | easy | base | cleared | 8/15 | 37 | 37 | 3 | 0 | 0 | 0 | -0.8 |
| classicplus-041 | KY's Constant | hard | base | cleared | 7/18 | 26 | 26 | 3 | 0 | 0 | 0 | -0.4 |
| classicplus-042 | KY's Test | easy | base | cleared | 9/13 | 13 | 13 | 6 | 0 | 0 | 0 | -1.3 |
| classicplus-042 | KY's Test | hard | base | cleared | 7/22 | 8 | 8 | 6 | 0 | 0 | 0 | -1.2 |
| classicplus-043 | AI Slop | easy | base | cleared | 7/11 | 30 | 30 | 3 | 0 | 0 | 0 | 0.0 |
| classicplus-043 | AI Slop | hard | base | cleared | 2/7 | 4 | 4 | 0 | 0 | 0 | 0 | n/a |
| classicplus-044 | Simplicity Audit | easy | base | cleared | 7/12 | 8 | 8 | 5 | 0 | 0 | 0 | 21.8 |
| classicplus-044 | Simplicity Audit | hard | base | cleared | 9/18 | 32 | 32 | 3 | 0 | 0 | 0 | 75.3 |
| classicplus-045 | Complexity Audit | easy | base | cleared | 3/8 | 14 | 14 | 3 | 0 | 0 | 0 | 58.8 |
| classicplus-045 | Complexity Audit | hard | base | cleared | 8/14 | 40 | 40 | 4 | 0 | 0 | 0 | 21.1 |
| classicplus-048 | Jlockeed's Lobbyist | easy | base | cleared | 17/29 | 44 | 41 | 18 | 0 | 0 | 0 | 2.1 |
| classicplus-048 | Jlockeed's Lobbyist | hard | base | cleared | 13/33 | 30 | 24 | 9 | 0 | 0 | 0 | 4.0 |
| classicplus-065 | Two Grapes | easy | base | cleared | 9/13 | 15 | 15 | 9 | 0 | 0 | 0 | 1.7 |
| classicplus-065 | Two Grapes | hard | base | cleared | 7/18 | 9 | 9 | 6 | 0 | 0 | 0 | 1.8 |
| classicplus-066 | Vine of Grapes | easy | base | cleared | 10/15 | 22 | 22 | 7 | 0 | 0 | 0 | 4.7 |
| classicplus-066 | Vine of Grapes | hard | base | cleared | 15/23 | 57 | 57 | 2 | 0 | 0 | 0 | 0.3 |
| classicplus-073 | Call to Chaos (Classic+ Edition) | easy | base | cleared | 13/18 | 12 | 12 | 8 | 0 | 0 | 0 | 10.2 |
| classicplus-073 | Call to Chaos (Classic+ Edition) | hard | base | cleared | 5/12 | 7 | 7 | 1 | 0 | 0 | 0 | 2.4 |
| classicplus-078 | Claude's Datacenter | easy | base | cleared | 8/12 | 30 | 29 | 1 | 0 | 0 | 0 | 2.0 |
| classicplus-078 | Claude's Datacenter | hard | base | cleared | 10/18 | 36 | 36 | 3 | 0 | 0 | 0 | 1.8 |
| core-001 | Big D-fender | easy | base | cleared | 10/23 | 24 | 19 | 8 | 0 | 0 | 0 | 4.9 |
| core-001 | Big D-fender | hard | base | cleared | 13/30 | 36 | 27 | 7 | 0 | 0 | 0 | 7.2 |
| core-005 | Stockpile | easy | base | cleared | 9/12 | 16 | 16 | 5 | 0 | 0 | 0 | 4.3 |
| core-005 | Stockpile | hard | base | cleared | 10/19 | 15 | 15 | 8 | 0 | 0 | 0 | 1.7 |
| core-006 | Mana Well | easy | base | cleared | 4/9 | 17 | 17 | 3 | 0 | 0 | 0 | 1.5 |
| core-006 | Mana Well | hard | base | cleared | 2/8 | 10 | 10 | 0 | 0 | 0 | 0 | n/a |
| core-007 | Jewelosco Scarab | easy | base | cleared | 13/27 | 17 | 17 | 11 | 0 | 0 | 0 | 2.8 |
| core-007 | Jewelosco Scarab | hard | base | cleared | 14/41 | 29 | 22 | 8 | 0 | 0 | 0 | 1.3 |
| core-016 | Hit Job | easy | base | cleared | 8/15 | 15 | 15 | 5 | 0 | 0 | 0 | 41.4 |
| core-016 | Hit Job | hard | base | cleared | 11/19 | 30 | 30 | 8 | 0 | 0 | 0 | 13.8 |
| core-017 | Flood | easy | base | cleared | 3/4 | 5 | 5 | 1 | 0 | 0 | 0 | 30.0 |
| core-017 | Flood | hard | base | cleared | 9/15 | 20 | 20 | 1 | 0 | 0 | 0 | 27.9 |
| core-022 | Carnivorous Cube | easy | base | cleared | 11/21 | 42 | 39 | 6 | 0 | 0 | 0 | 6.5 |
| core-022 | Carnivorous Cube | hard | base | cleared | 9/26 | 24 | 15 | 2 | 0 | 0 | 0 | 5.8 |
| core-026 | Glowy Jelly Bean | easy | base | cleared | 9/17 | 8 | 8 | 2 | 0 | 0 | 0 | 4.8 |
| core-026 | Glowy Jelly Bean | hard | base | cleared | 12/15 | 15 | 15 | 2 | 0 | 0 | 0 | 8.4 |
| core-034 | Collateral Damage | easy | base | cleared | 7/10 | 12 | 12 | 4 | 0 | 0 | 0 | 20.8 |
| core-034 | Collateral Damage | hard | base | cleared | 11/16 | 21 | 21 | 5 | 0 | 0 | 0 | 19.4 |
| core-040 | Echoes of the Forgotten | easy | base | cleared | 4/11 | 13 | 13 | 3 | 0 | 0 | 0 | 1.0 |
| core-040 | Echoes of the Forgotten | hard | base | cleared | 6/12 | 28 | 28 | 3 | 0 | 0 | 0 | 1.5 |
| core-042 | Selective Breeding | easy | base | cleared | 8/14 | 35 | 35 | 2 | 0 | 0 | 0 | -2.1 |
| core-042 | Selective Breeding | hard | base | cleared | 8/13 | 26 | 26 | 2 | 0 | 0 | 0 | -2.5 |
| core-048 | 5pek Controller | easy | base | cleared | 2/9 | 2 | 2 | 2 | 0 | 0 | 0 | 4.0 |
| core-048 | 5pek Controller | hard | base | cleared | 2/11 | 2 | 2 | 1 | 0 | 0 | 0 | 0.5 |
| core-057 | Conjure KY | easy | base | cleared | 9/18 | 20 | 20 | 7 | 0 | 0 | 0 | 3.2 |
| core-057 | Conjure KY | hard | base | cleared | 7/26 | 18 | 18 | 4 | 0 | 0 | 0 | 1.5 |
| core-058 | Rush Token Farm | easy | base | cleared | 3/9 | 3 | 3 | 3 | 0 | 0 | 0 | 1.0 |
| core-058 | Rush Token Farm | hard | base | cleared | 4/12 | 12 | 12 | 1 | 0 | 0 | 0 | -0.1 |
| core-059 | Unbiased Immigration | easy | base | cleared | 6/10 | 15 | 15 | 4 | 0 | 0 | 0 | 1.5 |
| core-059 | Unbiased Immigration | hard | base | cleared | 1/9 | 1 | 1 | 1 | 0 | 0 | 0 | 1.5 |
| core-065 | Masochism Mask | easy | base | cleared | 10/10 | 30 | 30 | 8 | 0 | 0 | 0 | -0.4 |
| core-065 | Masochism Mask | hard | base | cleared | 7/7 | 33 | 33 | 2 | 0 | 0 | 0 | 1.0 |
| core-069 | Call to Arms | easy | base | cleared | 3/9 | 3 | 3 | 2 | 0 | 0 | 0 | 12.3 |
| core-069 | Call to Arms | hard | base | cleared | 4/10 | 4 | 4 | 4 | 0 | 0 | 0 | 15.2 |
| core-072 | Reminisce | easy | base | cleared | 8/13 | 33 | 33 | 11 | 0 | 0 | 0 | -1.3 |
| core-072 | Reminisce | hard | base | cleared | 12/18 | 45 | 45 | 4 | 0 | 0 | 0 | -1.3 |
| core-079 | Twinspell | easy | base | cleared | 2/6 | 4 | 4 | 2 | 0 | 0 | 0 | 1.0 |
| core-079 | Twinspell | hard | base | cleared | 5/12 | 14 | 13 | 4 | 0 | 0 | 0 | 1.0 |
| core-083 | Transmogulate | easy | base | cleared | 5/12 | 30 | 30 | 3 | 0 | 0 | 0 | 35.5 |
| core-083 | Transmogulate | hard | base | cleared | 11/17 | 43 | 43 | 5 | 0 | 0 | 0 | 10.3 |
| core-086 | "Miss" Mrow | easy | base | cleared | 18/35 | 51 | 41 | 13 | 0 | 0 | 0 | -0.2 |
| core-086 | "Miss" Mrow | hard | base | cleared | 11/25 | 26 | 22 | 7 | 0 | 0 | 0 | -0.4 |
| core-087 | Pocket Chaos | easy | base | cleared | 2/4 | 4 | 4 | 2 | 0 | 0 | 0 | 73.8 |
| core-087 | Pocket Chaos | hard | base | cleared | 8/11 | 27 | 27 | 2 | 0 | 0 | 0 | 44.8 |
| core-090 | Super-Virus Injection | easy | base | cleared | 5/14 | 30 | 30 | 1 | 0 | 0 | 0 | -1.8 |
| core-090 | Super-Virus Injection | hard | base | cleared | 5/16 | 20 | 20 | 3 | 0 | 0 | 0 | -1.3 |
| core-092 | Felinor Fiender | easy | base | cleared | 14/20 | 20 | 20 | 12 | 0 | 0 | 0 | 15.3 |
| core-092 | Felinor Fiender | hard | base | cleared | 20/30 | 23 | 23 | 21 | 0 | 0 | 0 | 13.4 |
| core-093 | Combo-Index | easy | base | cleared | 7/16 | 20 | 20 | 3 | 0 | 0 | 0 | 3.8 |
| core-093 | Combo-Index | hard | base | cleared | 7/19 | 20 | 20 | 3 | 0 | 0 | 0 | 3.5 |
| core-094 | Genn's Greed | easy | base | cleared | 4/8 | 10 | 10 | 2 | 0 | 0 | 0 | -0.1 |
| core-094 | Genn's Greed | hard | base | cleared | 11/20 | 31 | 31 | 1 | 0 | 0 | 0 | n/a |
| core-099 | Craft a Card | easy | base | cleared | 4/7 | 17 | 17 | 0 | 0 | 0 | 0 | n/a |
| core-099 | Craft a Card | hard | base | cleared | 4/9 | 13 | 13 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-005 | Generated Filler | easy | base | cleared | 9/14 | 23 | 23 | 4 | 0 | 0 | 0 | 0.4 |
| infiniteassault-005 | Generated Filler | hard | base | cleared | 10/20 | 19 | 19 | 7 | 0 | 0 | 0 | -0.1 |
| infiniteassault-010 | Assault Carrier | easy | base | cleared | 8/12 | 11 | 9 | 8 | 0 | 0 | 0 | 4.8 |
| infiniteassault-010 | Assault Carrier | hard | base | cleared | 15/34 | 24 | 20 | 11 | 0 | 0 | 0 | 19.1 |
| infiniteassault-011 | Quickgen Turret | easy | base | cleared | 11/16 | 20 | 16 | 7 | 0 | 0 | 0 | 6.9 |
| infiniteassault-011 | Quickgen Turret | hard | base | cleared | 13/34 | 28 | 24 | 9 | 0 | 0 | 0 | 5.9 |
| infiniteassault-012 | Recompile | easy | base | cleared | 3/11 | 5 | 5 | 2 | 0 | 0 | 0 | 4.2 |
| infiniteassault-012 | Recompile | hard | base | cleared | 5/12 | 9 | 9 | 5 | 0 | 0 | 0 | 7.0 |
| infiniteassault-013 | Training Run | easy | base | cleared | 3/7 | 9 | 9 | 1 | 0 | 0 | 0 | -1.6 |
| infiniteassault-013 | Training Run | hard | base | cleared | 7/17 | 19 | 19 | 3 | 0 | 0 | 0 | -1.6 |
| infiniteassault-015 | Drone Foundry | easy | base | cleared | 8/11 | 19 | 18 | 5 | 0 | 0 | 0 | 1.0 |
| infiniteassault-015 | Drone Foundry | hard | base | cleared | 14/21 | 32 | 32 | 9 | 0 | 0 | 0 | 2.1 |
| infiniteassault-017 | Cluster Drone | easy | base | cleared | 8/18 | 12 | 7 | 6 | 0 | 0 | 0 | 5.5 |
| infiniteassault-017 | Cluster Drone | hard | base | cleared | 8/26 | 16 | 14 | 6 | 0 | 0 | 0 | 8.2 |
| infiniteassault-018 | Prompt Engineer | easy | base | cleared | 15/22 | 41 | 37 | 11 | 0 | 0 | 0 | 6.6 |
| infiniteassault-018 | Prompt Engineer | hard | base | cleared | 14/29 | 31 | 24 | 5 | 0 | 0 | 0 | 5.9 |
| infiniteassault-019 | Infinite Scroll | easy | base | cleared | 6/14 | 24 | 24 | 5 | 0 | 0 | 0 | 3.2 |
| infiniteassault-019 | Infinite Scroll | hard | base | cleared | 12/19 | 52 | 52 | 1 | 0 | 0 | 0 | 3.2 |
| infiniteassault-020 | Synthetic Data | easy | base | cleared | 8/12 | 21 | 21 | 5 | 0 | 0 | 0 | 3.8 |
| infiniteassault-020 | Synthetic Data | hard | base | cleared | 8/15 | 26 | 26 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-021 | Endless Production Line | easy | base | cleared | 2/7 | 3 | 3 | 2 | 0 | 0 | 0 | 2.5 |
| infiniteassault-021 | Endless Production Line | hard | base | cleared | 10/20 | 20 | 20 | 5 | 0 | 0 | 0 | 2.4 |
| infiniteassault-022 | Assault Mainframe | easy | base | cleared | 14/24 | 18 | 12 | 8 | 0 | 0 | 0 | 59.8 |
| infiniteassault-022 | Assault Mainframe | hard | base | cleared | 16/25 | 32 | 16 | 10 | 0 | 0 | 0 | 22.3 |
| infiniteassault-023 | The Generator | easy | base | cleared | 10/19 | 25 | 18 | 9 | 0 | 0 | 0 | 9.2 |
| infiniteassault-023 | The Generator | hard | base | cleared | 14/35 | 27 | 24 | 7 | 0 | 0 | 0 | 10.3 |
| infiniteassault-024 | Mass Deployment | easy | base | cleared | 4/5 | 3 | 3 | 2 | 0 | 0 | 0 | 29.5 |
| infiniteassault-024 | Mass Deployment | hard | base | cleared | 10/15 | 20 | 20 | 6 | 0 | 0 | 0 | 12.6 |
| infiniteassault-027 | Decoy Projector | easy | base | cleared | 15/25 | 11 | 11 | 13 | 0 | 0 | 0 | 21.1 |
| infiniteassault-027 | Decoy Projector | hard | base | cleared | 12/31 | 21 | 19 | 9 | 0 | 0 | 0 | 9.6 |
| infiniteassault-029 | False Positive | easy | base | cleared | 13/24 | 28 | 24 | 10 | 0 | 0 | 7 | 0.5 |
| infiniteassault-029 | False Positive | easy | radiant | cleared | 14/24 | 40 | 40 | 13 | 0 | 0 | 0 | 0.3 |
| infiniteassault-029 | False Positive | hard | base | cleared | 13/24 | 28 | 28 | 9 | 0 | 0 | 0 | 0.9 |
| infiniteassault-029 | False Positive | hard | radiant | cleared | 11/24 | 17 | 16 | 7 | 0 | 0 | 0 | -0.3 |
| infiniteassault-034 | Mass Hallucination | easy | base | cleared | 4/7 | 10 | 10 | 3 | 0 | 0 | 0 | 65.6 |
| infiniteassault-034 | Mass Hallucination | hard | base | cleared | 3/13 | 12 | 12 | 2 | 0 | 0 | 0 | 7.1 |
| infiniteassault-035 | Fata Morgana | easy | base | cleared | 21/37 | 27 | 23 | 16 | 0 | 0 | 1 | 8.5 |
| infiniteassault-035 | Fata Morgana | easy | radiant | cleared | 10/24 | 7 | 7 | 9 | 0 | 0 | 47 | 13.5 |
| infiniteassault-035 | Fata Morgana | hard | base | cleared | 20/45 | 44 | 33 | 16 | 0 | 0 | 0 | 8.6 |
| infiniteassault-035 | Fata Morgana | hard | radiant | cleared | 12/24 | 13 | 10 | 8 | 0 | 0 | 25 | 13.4 |
| infiniteassault-037 | Bluff Called | easy | base | cleared | 7/16 | 25 | 25 | 3 | 0 | 0 | 0 | -1.8 |
| infiniteassault-037 | Bluff Called | hard | base | cleared | 7/15 | 29 | 29 | 3 | 0 | 0 | 0 | 1.2 |
| infiniteassault-039 | Shell Game | easy | base | cleared | 11/22 | 20 | 20 | 8 | 0 | 0 | 0 | 2.5 |
| infiniteassault-039 | Shell Game | hard | base | cleared | 10/18 | 21 | 21 | 8 | 0 | 0 | 0 | 4.0 |
| infiniteassault-040 | Hall of Mirrors | easy | base | cleared | 7/13 | 38 | 38 | 5 | 0 | 0 | 0 | 1.8 |
| infiniteassault-040 | Hall of Mirrors | hard | base | cleared | 10/28 | 28 | 26 | 3 | 0 | 0 | 0 | 2.0 |
| infiniteassault-041 | Phantom Army | easy | base | cleared | 6/11 | 17 | 17 | 3 | 0 | 0 | 0 | 3.0 |
| infiniteassault-041 | Phantom Army | hard | base | cleared | 12/18 | 30 | 30 | 5 | 0 | 0 | 0 | 8.9 |
| infiniteassault-042 | Paranoia Engine | easy | base | cleared | 14/25 | 30 | 27 | 11 | 0 | 0 | 0 | 8.5 |
| infiniteassault-042 | Paranoia Engine | hard | base | cleared | 10/21 | 13 | 10 | 6 | 0 | 0 | 0 | 7.5 |
| infiniteassault-043 | Cognitive Dissonance | easy | base | cleared | 6/19 | 17 | 17 | 3 | 0 | 0 | 0 | 1.5 |
| infiniteassault-043 | Cognitive Dissonance | hard | base | cleared | 8/16 | 26 | 26 | 6 | 0 | 0 | 0 | 2.6 |
| infiniteassault-045 | Dream Within a Dream | easy | base | cleared | 5/8 | 9 | 9 | 3 | 0 | 0 | 0 | 8.0 |
| infiniteassault-045 | Dream Within a Dream | hard | base | cleared | 9/18 | 31 | 31 | 4 | 0 | 0 | 0 | 2.9 |
| infiniteassault-047 | Unreal Estate | easy | base | cleared | 5/9 | 16 | 16 | 4 | 0 | 0 | 0 | 1.4 |
| infiniteassault-047 | Unreal Estate | hard | base | cleared | 11/22 | 27 | 26 | 5 | 0 | 0 | 0 | 1.5 |
| infiniteassault-048 | Ego Death | easy | base | cleared | 27/39 | 118 | 118 | 10 | 0 | 0 | 33 | 8.1 |
| infiniteassault-048 | Ego Death | easy | radiant | neverPlayed | 16/24 | 54 | 54 | 8 | 0 | 0 | 0 | 8.5 |
| infiniteassault-048 | Ego Death | hard | base | cleared | 19/38 | 60 | 60 | 1 | 0 | 0 | 0 | 4.6 |
| infiniteassault-048 | Ego Death | hard | radiant | neverPlayed | 12/24 | 44 | 44 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-050 | Burst Clock | easy | base | cleared | 7/11 | 17 | 17 | 3 | 0 | 0 | 0 | -2.1 |
| infiniteassault-050 | Burst Clock | hard | base | cleared | 9/18 | 20 | 20 | 4 | 0 | 0 | 0 | -1.9 |
| infiniteassault-052 | Thermal Paste | easy | base | cleared | 7/11 | 18 | 18 | 4 | 0 | 0 | 0 | -3.8 |
| infiniteassault-052 | Thermal Paste | hard | base | cleared | 6/17 | 19 | 19 | 3 | 0 | 0 | 0 | -3.9 |
| infiniteassault-053 | Overvolt Grunt | easy | base | cleared | 16/28 | 28 | 21 | 13 | 0 | 0 | 0 | 7.8 |
| infiniteassault-053 | Overvolt Grunt | hard | base | cleared | 9/27 | 10 | 9 | 8 | 0 | 0 | 0 | 7.3 |
| infiniteassault-056 | Afterburner | easy | base | cleared | 13/20 | 29 | 29 | 8 | 0 | 0 | 0 | 5.5 |
| infiniteassault-056 | Afterburner | hard | base | cleared | 11/23 | 29 | 29 | 7 | 0 | 0 | 0 | 4.4 |
| infiniteassault-058 | Overclocked Core | easy | base | cleared | 11/24 | 24 | 16 | 7 | 0 | 0 | 0 | 6.7 |
| infiniteassault-058 | Overclocked Core | hard | base | cleared | 17/40 | 49 | 34 | 7 | 0 | 0 | 0 | 6.9 |
| infiniteassault-061 | Meltdown Charge | easy | base | cleared | 3/7 | 12 | 12 | 1 | 0 | 0 | 0 | n/a |
| infiniteassault-061 | Meltdown Charge | hard | base | cleared | 10/19 | 27 | 27 | 6 | 0 | 0 | 0 | 21.4 |
| infiniteassault-062 | Burnout | easy | base | cleared | 10/14 | 21 | 21 | 9 | 0 | 0 | 0 | 0.7 |
| infiniteassault-062 | Burnout | hard | base | cleared | 10/22 | 23 | 22 | 8 | 0 | 0 | 0 | 1.0 |
| infiniteassault-065 | Overdrive | easy | base | cleared | 5/8 | 11 | 11 | 3 | 0 | 0 | 0 | -1.7 |
| infiniteassault-065 | Overdrive | hard | base | cleared | 10/25 | 33 | 33 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-067 | Burning Ambition | easy | base | cleared | 3/13 | 11 | 11 | 2 | 0 | 0 | 0 | 1.0 |
| infiniteassault-067 | Burning Ambition | hard | base | cleared | 6/12 | 12 | 12 | 2 | 0 | 0 | 0 | 1.5 |
| infiniteassault-068 | Short Circuit | easy | base | cleared | 6/11 | 15 | 15 | 4 | 0 | 0 | 0 | 2.7 |
| infiniteassault-068 | Short Circuit | hard | base | cleared | 10/17 | 30 | 29 | 3 | 0 | 0 | 0 | 1.0 |
| infiniteassault-071 | Thermal Throttler | easy | base | cleared | 10/21 | 12 | 12 | 11 | 0 | 0 | 0 | 9.4 |
| infiniteassault-071 | Thermal Throttler | hard | base | cleared | 7/28 | 11 | 10 | 5 | 0 | 0 | 0 | 9.5 |
| infiniteassault-072 | Overclock to Infinity | easy | base | cleared | 19/32 | 44 | 44 | 9 | 0 | 0 | 2 | -2.2 |
| infiniteassault-072 | Overclock to Infinity | easy | radiant | cleared | 11/24 | 22 | 22 | 6 | 0 | 0 | 1 | -3.1 |
| infiniteassault-072 | Overclock to Infinity | hard | base | cleared | 22/48 | 26 | 26 | 15 | 0 | 0 | 2 | -2.2 |
| infiniteassault-072 | Overclock to Infinity | hard | radiant | cleared | 8/24 | 18 | 17 | 5 | 0 | 0 | 9 | -2.7 |
| infiniteassault-078 | Batch Job | easy | base | cleared | 8/13 | 20 | 20 | 5 | 0 | 0 | 0 | 10.6 |
| infiniteassault-078 | Batch Job | hard | base | cleared | 12/22 | 41 | 41 | 2 | 0 | 0 | 0 | 1.3 |
| infiniteassault-079 | Thread Pool | easy | base | cleared | 16/29 | 32 | 31 | 11 | 0 | 0 | 0 | 15.8 |
| infiniteassault-079 | Thread Pool | hard | base | cleared | 19/50 | 40 | 39 | 11 | 0 | 0 | 0 | 11.9 |
| infiniteassault-084 | Distributed Mind | easy | base | cleared | 19/34 | 50 | 39 | 13 | 0 | 0 | 0 | 20.4 |
| infiniteassault-084 | Distributed Mind | hard | base | cleared | 15/41 | 39 | 28 | 4 | 0 | 0 | 0 | 6.3 |
| infiniteassault-085 | Merge Conflict | easy | base | cleared | 7/8 | 30 | 19 | 3 | 0 | 0 | 0 | 45.4 |
| infiniteassault-085 | Merge Conflict | hard | base | cleared | 9/18 | 35 | 24 | 4 | 0 | 0 | 0 | 21.0 |
| infiniteassault-086 | Replica Hatchery | easy | base | cleared | 5/6 | 12 | 12 | 2 | 0 | 0 | 0 | 0.7 |
| infiniteassault-086 | Replica Hatchery | hard | base | cleared | 6/16 | 21 | 15 | 4 | 0 | 0 | 0 | 1.5 |
| infiniteassault-088 | Scale Out | easy | base | cleared | 10/12 | 26 | 26 | 5 | 0 | 0 | 0 | 9.9 |
| infiniteassault-088 | Scale Out | hard | base | cleared | 10/17 | 41 | 41 | 4 | 0 | 0 | 0 | 5.7 |
| infiniteassault-090 | Mass Production | easy | base | cleared | 3/6 | 7 | 7 | 3 | 0 | 0 | 0 | 22.6 |
| infiniteassault-090 | Mass Production | hard | base | cleared | 11/26 | 21 | 21 | 9 | 0 | 0 | 0 | 21.4 |
| infiniteassault-091 | Flood the Zone | easy | base | cleared | 6/11 | 14 | 14 | 3 | 0 | 0 | 0 | 3.5 |
| infiniteassault-091 | Flood the Zone | hard | base | cleared | 12/25 | 35 | 35 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-092 | Hive Overseer | easy | base | cleared | 15/24 | 26 | 20 | 12 | 0 | 0 | 0 | 24.2 |
| infiniteassault-092 | Hive Overseer | hard | base | cleared | 22/49 | 64 | 48 | 11 | 0 | 0 | 0 | 26.4 |
| infiniteassault-093 | Gestalt | easy | base | cleared | 9/18 | 24 | 18 | 2 | 0 | 0 | 0 | 1.1 |
| infiniteassault-093 | Gestalt | hard | base | cleared | 18/41 | 60 | 43 | 4 | 0 | 0 | 0 | -0.2 |
| infiniteassault-095 | The Botnet | easy | base | cleared | 4/8 | 7 | 7 | 3 | 0 | 0 | 0 | 23.3 |
| infiniteassault-095 | The Botnet | hard | base | cleared | 15/25 | 27 | 28 | 15 | 0 | 0 | 0 | 32.2 |
| infiniteassault-098 | Echo Chamber | easy | base | cleared | 11/13 | 23 | 23 | 12 | 0 | 0 | 0 | 14.9 |
| infiniteassault-098 | Echo Chamber | hard | base | cleared | 16/25 | 33 | 33 | 8 | 0 | 0 | 0 | 1.8 |
| infiniteassault-099 | Recall | easy | base | cleared | 11/18 | 36 | 36 | 9 | 0 | 0 | 0 | 0.1 |
| infiniteassault-099 | Recall | hard | base | cleared | 4/14 | 15 | 15 | 2 | 0 | 0 | 0 | -0.7 |
| infiniteassault-100 | Iterator | easy | base | cleared | 8/22 | 26 | 25 | 4 | 0 | 0 | 0 | 5.6 |
| infiniteassault-100 | Iterator | hard | base | cleared | 23/39 | 50 | 43 | 12 | 0 | 0 | 0 | 6.9 |
| infiniteassault-103 | Undo | easy | base | cleared | 7/10 | 11 | 11 | 7 | 0 | 0 | 0 | 12.0 |
| infiniteassault-103 | Undo | hard | base | cleared | 4/14 | 13 | 13 | 5 | 0 | 0 | 0 | 9.8 |
| infiniteassault-105 | Callback | easy | base | cleared | 14/26 | 21 | 21 | 12 | 0 | 0 | 0 | 7.9 |
| infiniteassault-105 | Callback | hard | base | cleared | 10/33 | 26 | 23 | 5 | 0 | 0 | 0 | 11.0 |
| infiniteassault-106 | Checkpoint Daemon | easy | base | cleared | 19/31 | 72 | 57 | 8 | 0 | 0 | 0 | 6.9 |
| infiniteassault-106 | Checkpoint Daemon | hard | base | cleared | 18/39 | 53 | 44 | 5 | 0 | 0 | 0 | 7.1 |
| infiniteassault-107 | Recursive Call | easy | base | cleared | 4/6 | 27 | 27 | 3 | 0 | 0 | 0 | -1.4 |
| infiniteassault-107 | Recursive Call | hard | base | cleared | 4/8 | 13 | 13 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-108 | Second Draft | easy | base | cleared | 8/16 | 14 | 14 | 7 | 0 | 0 | 0 | 0.7 |
| infiniteassault-108 | Second Draft | hard | base | cleared | 3/8 | 4 | 4 | 1 | 0 | 0 | 0 | 0.2 |
| infiniteassault-109 | Revenant Process | easy | base | cleared | 16/28 | 28 | 26 | 12 | 0 | 0 | 0 | 8.7 |
| infiniteassault-109 | Revenant Process | hard | base | cleared | 16/31 | 29 | 27 | 15 | 0 | 0 | 0 | 10.0 |
| infiniteassault-110 | Infinite Loop | easy | base | cleared | 1/13 | 5 | 5 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-110 | Infinite Loop | hard | base | cleared | 11/24 | 30 | 26 | 4 | 0 | 0 | 0 | 2.4 |
| infiniteassault-111 | Redundant Copy | easy | base | cleared | 9/19 | 16 | 16 | 8 | 0 | 0 | 0 | 0.8 |
| infiniteassault-111 | Redundant Copy | hard | base | cleared | 8/18 | 19 | 19 | 4 | 0 | 0 | 0 | 0.6 |
| infiniteassault-112 | Evolutionary Loop | easy | base | cleared | 13/27 | 39 | 33 | 11 | 0 | 0 | 0 | 4.8 |
| infiniteassault-112 | Evolutionary Loop | hard | base | cleared | 12/31 | 41 | 28 | 4 | 0 | 0 | 0 | 4.7 |
| infiniteassault-113 | Rerun | easy | base | cleared | 6/14 | 13 | 13 | 5 | 0 | 0 | 0 | 35.9 |
| infiniteassault-113 | Rerun | hard | base | cleared | 13/29 | 39 | 39 | 6 | 0 | 0 | 0 | 7.8 |
| infiniteassault-114 | Ouroboros Engine | easy | base | cleared | 11/23 | 24 | 19 | 9 | 0 | 0 | 0 | 11.0 |
| infiniteassault-114 | Ouroboros Engine | hard | base | cleared | 29/49 | 53 | 37 | 16 | 0 | 0 | 0 | 12.2 |
| infiniteassault-116 | Groundhog Day | easy | base | cleared | 7/10 | 12 | 12 | 3 | 0 | 0 | 0 | 10.7 |
| infiniteassault-116 | Groundhog Day | hard | base | cleared | 7/19 | 21 | 21 | 2 | 0 | 0 | 0 | 5.1 |
| infiniteassault-117 | Core Dump | easy | base | cleared | 8/13 | 20 | 20 | 4 | 0 | 0 | 0 | 1.1 |
| infiniteassault-117 | Core Dump | hard | base | cleared | 10/20 | 31 | 30 | 3 | 0 | 0 | 0 | 1.5 |
| infiniteassault-118 | The Recursionist | easy | base | cleared | 15/30 | 23 | 22 | 11 | 0 | 0 | 0 | 11.3 |
| infiniteassault-118 | The Recursionist | hard | base | cleared | 14/38 | 30 | 22 | 7 | 0 | 0 | 0 | 13.0 |
| infiniteassault-119 | Snapshot Lich | easy | base | cleared | 9/21 | 13 | 12 | 7 | 0 | 0 | 0 | 22.7 |
| infiniteassault-119 | Snapshot Lich | hard | base | cleared | 18/34 | 24 | 18 | 10 | 0 | 0 | 0 | 17.4 |
| infiniteassault-120 | Eternal Return | easy | base | cleared | 3/11 | 11 | 11 | 1 | 0 | 0 | 0 | 10.0 |
| infiniteassault-120 | Eternal Return | hard | base | cleared | 11/24 | 36 | 36 | 2 | 0 | 0 | 0 | 1.7 |
| infiniteassault-123 | Cost Injection | easy | base | cleared | 11/24 | 48 | 48 | 2 | 0 | 0 | 4 | -1.2 |
| infiniteassault-123 | Cost Injection | easy | radiant | cleared | 13/24 | 83 | 83 | 2 | 0 | 0 | 19 | -2.7 |
| infiniteassault-123 | Cost Injection | hard | base | cleared | 12/24 | 35 | 35 | 3 | 0 | 0 | 13 | -0.9 |
| infiniteassault-123 | Cost Injection | hard | radiant | cleared | 14/24 | 64 | 64 | 3 | 0 | 0 | 1 | -2.3 |
| infiniteassault-124 | Firewall Breach | easy | base | cleared | 7/13 | 37 | 37 | 3 | 0 | 0 | 0 | 0.1 |
| infiniteassault-124 | Firewall Breach | hard | base | cleared | 10/17 | 39 | 39 | 1 | 0 | 0 | 0 | -0.3 |
| infiniteassault-125 | Zone Lock | easy | base | cleared | 8/16 | 12 | 12 | 7 | 0 | 0 | 0 | -0.4 |
| infiniteassault-125 | Zone Lock | hard | base | cleared | 6/15 | 10 | 10 | 2 | 0 | 0 | 0 | 0.6 |
| infiniteassault-127 | Bad Prompt | easy | base | cleared | 10/12 | 68 | 68 | 1 | 0 | 0 | 0 | -1.6 |
| infiniteassault-127 | Bad Prompt | hard | base | cleared | 11/21 | 40 | 40 | 1 | 0 | 0 | 0 | -1.6 |
| infiniteassault-129 | Signal Jammer | easy | base | cleared | 11/27 | 11 | 10 | 8 | 0 | 0 | 0 | 7.8 |
| infiniteassault-129 | Signal Jammer | hard | base | cleared | 11/33 | 26 | 20 | 4 | 0 | 0 | 0 | 8.4 |
| infiniteassault-130 | Prompt Injection | easy | base | cleared | 9/13 | 28 | 28 | 2 | 0 | 0 | 0 | 2.2 |
| infiniteassault-130 | Prompt Injection | hard | base | cleared | 13/23 | 55 | 54 | 1 | 0 | 0 | 0 | 2.8 |
| infiniteassault-132 | Ransomware | easy | base | cleared | 11/14 | 37 | 36 | 6 | 0 | 0 | 0 | 1.5 |
| infiniteassault-132 | Ransomware | hard | base | cleared | 7/18 | 20 | 20 | 3 | 0 | 0 | 0 | 1.2 |
| infiniteassault-134 | Rootkit Runner | easy | base | cleared | 9/19 | 28 | 28 | 6 | 0 | 0 | 0 | 6.6 |
| infiniteassault-134 | Rootkit Runner | hard | base | cleared | 12/27 | 26 | 26 | 8 | 0 | 0 | 0 | 5.0 |
| infiniteassault-135 | Mana Leech | easy | base | cleared | 14/25 | 26 | 18 | 11 | 0 | 0 | 0 | 5.4 |
| infiniteassault-135 | Mana Leech | hard | base | cleared | 8/27 | 14 | 14 | 7 | 0 | 0 | 0 | 5.7 |
| infiniteassault-136 | DDoS | easy | base | cleared | 7/12 | 20 | 20 | 4 | 0 | 0 | 0 | 9.1 |
| infiniteassault-136 | DDoS | hard | base | cleared | 5/17 | 12 | 12 | 3 | 0 | 0 | 0 | 64.3 |
| infiniteassault-137 | Patch Tuesday Exploit | easy | base | cleared | 4/12 | 11 | 11 | 2 | 0 | 0 | 0 | -1.6 |
| infiniteassault-137 | Patch Tuesday Exploit | hard | base | cleared | 6/13 | 28 | 28 | 2 | 0 | 0 | 0 | -1.8 |
| infiniteassault-138 | Social Engineer | easy | base | cleared | 13/23 | 24 | 19 | 9 | 0 | 0 | 0 | 12.2 |
| infiniteassault-138 | Social Engineer | hard | base | cleared | 16/30 | 34 | 25 | 6 | 0 | 0 | 0 | 55.6 |
| infiniteassault-140 | Kill Switch | easy | base | cleared | 7/15 | 21 | 20 | 3 | 0 | 0 | 0 | 1.7 |
| infiniteassault-140 | Kill Switch | hard | base | cleared | 14/25 | 38 | 38 | 3 | 0 | 0 | 0 | 2.0 |
| infiniteassault-141 | Logic Bomb | easy | base | cleared | 11/22 | 36 | 30 | 8 | 0 | 0 | 0 | 4.9 |
| infiniteassault-141 | Logic Bomb | hard | base | cleared | 15/38 | 43 | 36 | 4 | 0 | 0 | 0 | 4.4 |
| infiniteassault-143 | Zero-Click | easy | base | cleared | 7/8 | 9 | 9 | 6 | 0 | 0 | 0 | 34.8 |
| infiniteassault-143 | Zero-Click | hard | base | cleared | 9/15 | 27 | 27 | 7 | 0 | 0 | 0 | 18.1 |
| infiniteassault-144 | Total Compromise | easy | base | cleared | 8/10 | 18 | 18 | 5 | 0 | 0 | 0 | 56.1 |
| infiniteassault-144 | Total Compromise | hard | base | cleared | 12/31 | 18 | 18 | 8 | 0 | 0 | 0 | 27.6 |
| infiniteassault-146 | Shield Drone | easy | base | cleared | 9/19 | 16 | 15 | 6 | 0 | 0 | 0 | 3.3 |
| infiniteassault-146 | Shield Drone | hard | base | cleared | 16/29 | 33 | 20 | 11 | 0 | 0 | 0 | 3.7 |
| infiniteassault-148 | Quarantine | easy | base | cleared | 8/21 | 22 | 22 | 7 | 0 | 0 | 0 | 0.8 |
| infiniteassault-148 | Quarantine | hard | base | cleared | 6/13 | 19 | 17 | 5 | 0 | 0 | 0 | 1.0 |
| infiniteassault-149 | Encrypt | easy | base | timeout | 14/24 | 31 | 31 | 13 | 0 | 0 | 2 | 4.6 |
| infiniteassault-149 | Encrypt | easy | radiant | cleared | 16/24 | 26 | 26 | 15 | 0 | 0 | 7 | 7.4 |
| infiniteassault-149 | Encrypt | hard | base | timeout | 13/24 | 21 | 21 | 13 | 0 | 0 | 4 | 6.5 |
| infiniteassault-149 | Encrypt | hard | radiant | cleared | 10/24 | 10 | 10 | 9 | 0 | 0 | 0 | 7.3 |
| infiniteassault-150 | Wall of Static | easy | base | cleared | 11/25 | 17 | 15 | 9 | 0 | 0 | 0 | 27.8 |
| infiniteassault-150 | Wall of Static | hard | base | cleared | 16/30 | 40 | 32 | 10 | 0 | 0 | 0 | 24.9 |
| infiniteassault-154 | Honeypot Protocol | easy | base | cleared | 7/9 | 18 | 18 | 4 | 0 | 0 | 0 | 1.5 |
| infiniteassault-154 | Honeypot Protocol | hard | base | cleared | 5/14 | 12 | 12 | 2 | 0 | 0 | 0 | 1.5 |
| infiniteassault-157 | Containment Field | easy | base | cleared | 5/9 | 28 | 28 | 3 | 0 | 0 | 0 | 69.0 |
| infiniteassault-157 | Containment Field | hard | base | cleared | 7/16 | 26 | 26 | 4 | 0 | 0 | 0 | 5.5 |
| infiniteassault-158 | Access Denied | easy | base | cleared | 8/12 | 11 | 11 | 6 | 0 | 0 | 0 | 1.3 |
| infiniteassault-158 | Access Denied | hard | base | cleared | 10/18 | 27 | 25 | 7 | 0 | 0 | 0 | 0.8 |
| infiniteassault-160 | Zero Trust | easy | base | cleared | 7/14 | 23 | 22 | 6 | 0 | 0 | 0 | 6.1 |
| infiniteassault-160 | Zero Trust | hard | base | cleared | 10/24 | 34 | 33 | 4 | 0 | 0 | 0 | 5.4 |
| infiniteassault-161 | Defense in Depth | easy | base | cleared | 14/30 | 37 | 37 | 12 | 0 | 0 | 0 | 17.9 |
| infiniteassault-161 | Defense in Depth | easy | radiant | cleared | 12/24 | 14 | 14 | 10 | 0 | 0 | 0 | 10.4 |
| infiniteassault-161 | Defense in Depth | hard | base | cleared | 16/38 | 35 | 35 | 10 | 0 | 0 | 14 | 4.4 |
| infiniteassault-161 | Defense in Depth | hard | radiant | cleared | 7/24 | 13 | 13 | 7 | 0 | 0 | 2 | 10.4 |
| infiniteassault-164 | Blackhole Route | easy | base | cleared | 6/13 | 15 | 15 | 6 | 0 | 0 | 0 | 1.2 |
| infiniteassault-164 | Blackhole Route | hard | base | cleared | 11/18 | 28 | 25 | 4 | 0 | 0 | 0 | 1.5 |
| infiniteassault-165 | Iron Curtain | easy | base | cleared | 12/15 | 27 | 26 | 7 | 0 | 0 | 0 | 1.7 |
| infiniteassault-165 | Iron Curtain | hard | base | cleared | 9/19 | 30 | 29 | 3 | 0 | 0 | 0 | 2.0 |
| infiniteassault-166 | Aegis Prime | easy | base | cleared | 14/19 | 12 | 11 | 10 | 0 | 0 | 0 | 39.1 |
| infiniteassault-166 | Aegis Prime | hard | base | cleared | 15/41 | 21 | 19 | 11 | 0 | 0 | 0 | 23.7 |
| infiniteassault-167 | Fortress Protocol | easy | base | cleared | 5/11 | 12 | 12 | 2 | 0 | 0 | 0 | -1.2 |
| infiniteassault-167 | Fortress Protocol | hard | base | cleared | 16/23 | 56 | 56 | 11 | 0 | 0 | 0 | 12.8 |
| infiniteassault-168 | Impenetrable | easy | base | cleared | 3/9 | 10 | 10 | 2 | 0 | 0 | 0 | 74.4 |
| infiniteassault-168 | Impenetrable | hard | base | cleared | 8/24 | 34 | 34 | 2 | 0 | 0 | 0 | 75.3 |
| infiniteassault-169 | Spider Bot | easy | base | cleared | 11/22 | 13 | 13 | 9 | 0 | 0 | 0 | 5.1 |
| infiniteassault-169 | Spider Bot | hard | base | cleared | 12/32 | 28 | 20 | 6 | 0 | 0 | 0 | 5.4 |
| infiniteassault-171 | Mimic Unit | easy | base | cleared | 7/15 | 21 | 18 | 4 | 0 | 0 | 0 | 16.1 |
| infiniteassault-171 | Mimic Unit | hard | base | cleared | 13/37 | 40 | 30 | 6 | 0 | 0 | 0 | 8.6 |
| infiniteassault-172 | Training Data | easy | base | cleared | 6/12 | 29 | 29 | 2 | 0 | 0 | 0 | 2.7 |
| infiniteassault-172 | Training Data | hard | base | cleared | 7/15 | 33 | 33 | 3 | 0 | 0 | 0 | 1.6 |
| infiniteassault-174 | Scrape the Graveyard | easy | base | cleared | 5/11 | 15 | 15 | 3 | 0 | 0 | 0 | 0.8 |
| infiniteassault-174 | Scrape the Graveyard | hard | base | cleared | 7/20 | 31 | 31 | 1 | 0 | 0 | 0 | 1.3 |
| infiniteassault-175 | Fingerprinter | easy | base | cleared | 8/18 | 20 | 16 | 4 | 0 | 0 | 0 | 4.6 |
| infiniteassault-175 | Fingerprinter | hard | base | cleared | 13/26 | 19 | 17 | 10 | 0 | 0 | 0 | 5.6 |
| infiniteassault-176 | Imitation Engine | easy | base | cleared | 16/30 | 32 | 26 | 10 | 0 | 0 | 0 | 13.7 |
| infiniteassault-176 | Imitation Engine | hard | base | cleared | 17/32 | 34 | 28 | 6 | 0 | 0 | 0 | 13.2 |
| infiniteassault-177 | Predictive Text | easy | base | cleared | 9/12 | 31 | 31 | 5 | 0 | 0 | 0 | 2.2 |
| infiniteassault-177 | Predictive Text | hard | base | cleared | 9/19 | 29 | 29 | 4 | 0 | 0 | 0 | 0.3 |
| infiniteassault-178 | Dataset Thief | easy | base | cleared | 16/24 | 45 | 32 | 11 | 0 | 0 | 0 | 13.0 |
| infiniteassault-178 | Dataset Thief | hard | base | cleared | 16/38 | 30 | 21 | 9 | 0 | 0 | 0 | 12.2 |
| infiniteassault-179 | Style Transfer | easy | base | cleared | 8/11 | 22 | 22 | 8 | 0 | 0 | 0 | 16.6 |
| infiniteassault-179 | Style Transfer | hard | base | cleared | 8/14 | 31 | 31 | 5 | 0 | 0 | 0 | 9.5 |
| infiniteassault-182 | Model Theft | easy | base | cleared | 6/8 | 11 | 11 | 3 | 0 | 0 | 0 | 19.2 |
| infiniteassault-182 | Model Theft | hard | base | cleared | 9/18 | 17 | 17 | 6 | 0 | 0 | 0 | 7.8 |
| infiniteassault-183 | Shadow Library | easy | base | cleared | 6/11 | 11 | 11 | 5 | 0 | 0 | 0 | 1.0 |
| infiniteassault-183 | Shadow Library | hard | base | cleared | 9/20 | 22 | 21 | 2 | 0 | 0 | 0 | 1.5 |
| infiniteassault-184 | Pattern Recognition | easy | base | cleared | 6/10 | 21 | 21 | 4 | 0 | 0 | 0 | -0.7 |
| infiniteassault-184 | Pattern Recognition | hard | base | cleared | 7/13 | 21 | 21 | 5 | 0 | 0 | 0 | -1.2 |
| infiniteassault-185 | Deep Crawler | easy | base | cleared | 21/32 | 35 | 30 | 16 | 0 | 0 | 0 | 14.3 |
| infiniteassault-185 | Deep Crawler | hard | base | cleared | 18/38 | 38 | 34 | 10 | 0 | 0 | 0 | 14.7 |
| infiniteassault-186 | Identity Theft | easy | base | cleared | 11/18 | 43 | 43 | 5 | 0 | 0 | 0 | 5.5 |
| infiniteassault-186 | Identity Theft | hard | base | cleared | 7/12 | 28 | 28 | 3 | 0 | 0 | 0 | 0.7 |
| infiniteassault-187 | Fine-Tuner | easy | base | cleared | 9/23 | 14 | 11 | 5 | 0 | 0 | 0 | 6.9 |
| infiniteassault-187 | Fine-Tuner | hard | base | cleared | 14/32 | 28 | 21 | 5 | 0 | 0 | 0 | 7.6 |
| infiniteassault-188 | Overfitting | easy | base | cleared | 11/15 | 39 | 39 | 4 | 0 | 0 | 0 | 1.9 |
| infiniteassault-188 | Overfitting | hard | base | cleared | 9/20 | 28 | 27 | 2 | 0 | 0 | 0 | 2.0 |
| infiniteassault-189 | Clone Vat | easy | base | cleared | 7/9 | 13 | 13 | 6 | 0 | 0 | 0 | 3.0 |
| infiniteassault-189 | Clone Vat | hard | base | cleared | 14/25 | 45 | 39 | 2 | 0 | 0 | 0 | 2.5 |
| infiniteassault-191 | Ground Truth | easy | base | cleared | 10/19 | 13 | 12 | 8 | 0 | 0 | 0 | 7.1 |
| infiniteassault-191 | Ground Truth | hard | base | cleared | 13/29 | 35 | 28 | 4 | 0 | 0 | 0 | 7.7 |
| infiniteassault-192 | Become the Data | easy | base | cleared | 6/10 | 14 | 14 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-192 | Become the Data | hard | base | cleared | 7/17 | 22 | 22 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-194 | Compute Shard | easy | base | cleared | 16/29 | 32 | 22 | 10 | 0 | 0 | 0 | 1.7 |
| infiniteassault-194 | Compute Shard | hard | base | cleared | 12/29 | 19 | 17 | 7 | 0 | 0 | 0 | 1.7 |
| infiniteassault-195 | Power Law | easy | base | cleared | 12/26 | 20 | 19 | 9 | 0 | 0 | 0 | 16.7 |
| infiniteassault-195 | Power Law | hard | base | cleared | 18/34 | 34 | 26 | 12 | 0 | 0 | 0 | 10.1 |
| infiniteassault-196 | Emergent Behavior | easy | base | cleared | 3/10 | 4 | 4 | 3 | 0 | 0 | 0 | 4.4 |
| infiniteassault-196 | Emergent Behavior | hard | base | cleared | 6/14 | 17 | 17 | 5 | 0 | 0 | 0 | 10.4 |
| infiniteassault-199 | Patient Machine | easy | base | cleared | 11/19 | 16 | 15 | 9 | 0 | 0 | 0 | 3.9 |
| infiniteassault-199 | Patient Machine | hard | base | cleared | 12/28 | 44 | 27 | 3 | 0 | 0 | 0 | 4.4 |
| infiniteassault-202 | Exponent | easy | base | cleared | 19/26 | 70 | 57 | 7 | 0 | 0 | 0 | 3.5 |
| infiniteassault-202 | Exponent | hard | base | cleared | 14/34 | 38 | 34 | 6 | 0 | 0 | 0 | 2.8 |
| infiniteassault-203 | Compute Cluster | easy | base | cleared | 8/13 | 18 | 18 | 7 | 0 | 0 | 0 | 1.5 |
| infiniteassault-203 | Compute Cluster | hard | base | cleared | 5/14 | 11 | 11 | 3 | 0 | 0 | 0 | 1.0 |
| infiniteassault-207 | Recursive Self-Improvement | easy | base | cleared | 5/9 | 8 | 8 | 5 | 0 | 0 | 0 | 3.4 |
| infiniteassault-207 | Recursive Self-Improvement | hard | base | cleared | 7/13 | 23 | 23 | 6 | 0 | 0 | 0 | 4.5 |
| infiniteassault-211 | Grey Goo | easy | base | cleared | 27/45 | 115 | 98 | 4 | 0 | 0 | 0 | 0.8 |
| infiniteassault-211 | Grey Goo | easy | radiant | neverPlayed | 8/24 | 38 | 28 | 3 | 0 | 0 | 0 | 1.0 |
| infiniteassault-211 | Grey Goo | hard | base | cleared | 30/57 | 115 | 80 | 2 | 0 | 0 | 0 | 1.4 |
| infiniteassault-211 | Grey Goo | hard | radiant | neverPlayed | 11/24 | 43 | 41 | 0 | 0 | 0 | 0 | n/a |
| infiniteassault-213 | The Long Game | easy | base | cleared | 16/25 | 36 | 33 | 11 | 0 | 0 | 0 | 1.1 |
| infiniteassault-213 | The Long Game | hard | base | cleared | 14/40 | 34 | 27 | 4 | 0 | 0 | 0 | 1.8 |
| infiniteassault-214 | Emergent Mind | easy | base | cleared | 14/26 | 19 | 17 | 4 | 0 | 0 | 0 | 8.9 |
| infiniteassault-214 | Emergent Mind | hard | base | cleared | 19/37 | 40 | 34 | 7 | 0 | 0 | 0 | 8.4 |
| infiniteassault-218 | Thousand Faces | easy | base | cleared | 5/13 | 30 | 30 | 1 | 0 | 0 | 0 | 1.5 |
| infiniteassault-218 | Thousand Faces | hard | base | cleared | 5/13 | 14 | 14 | 2 | 0 | 0 | 0 | 1.5 |
| infiniteassault-220 | Last Lap | easy | base | cleared | 5/14 | 18 | 18 | 3 | 0 | 0 | 0 | 22.6 |
| infiniteassault-220 | Last Lap | hard | base | cleared | 11/20 | 25 | 25 | 10 | 0 | 0 | 0 | 13.1 |
| infiniteassault-223 | Loopback, Who Returns | easy | base | cleared | 10/17 | 12 | 10 | 10 | 0 | 0 | 0 | 15.5 |
| infiniteassault-223 | Loopback, Who Returns | hard | base | cleared | 14/32 | 31 | 22 | 7 | 0 | 0 | 0 | 17.0 |
| infiniteassault-224 | Rollback Point | easy | base | cleared | 4/9 | 10 | 10 | 4 | 0 | 0 | 0 | 14.7 |
| infiniteassault-224 | Rollback Point | hard | base | cleared | 16/24 | 44 | 44 | 6 | 0 | 0 | 0 | 22.3 |
| infiniteassault-226 | Escalated Privileges | easy | base | cleared | 6/14 | 17 | 17 | 4 | 0 | 0 | 0 | 4.5 |
| infiniteassault-226 | Escalated Privileges | hard | base | cleared | 8/19 | 22 | 22 | 2 | 0 | 0 | 0 | 5.4 |
| infiniteassault-228 | Firewall Rules | easy | base | cleared | 7/11 | 34 | 30 | 3 | 0 | 0 | 0 | -1.3 |
| infiniteassault-228 | Firewall Rules | hard | base | cleared | 10/26 | 32 | 30 | 5 | 0 | 0 | 0 | 2.0 |
| infiniteassault-234 | Degenerate Output | easy | base | cleared | 7/10 | 12 | 12 | 6 | 0 | 0 | 0 | 41.3 |
| infiniteassault-234 | Degenerate Output | hard | base | cleared | 11/23 | 26 | 26 | 6 | 0 | 0 | 0 | 9.4 |
| infiniteassault-236 | Infinite Assault | easy | base | cleared | 10/14 | 14 | 14 | 7 | 0 | 0 | 0 | 47.6 |
| infiniteassault-236 | Infinite Assault | hard | base | cleared | 10/18 | 14 | 14 | 8 | 0 | 0 | 0 | 40.5 |
| infiniteassault-237 | Thermal Runaway | easy | base | cleared | 4/13 | 4 | 4 | 3 | 0 | 0 | 0 | 28.6 |
| infiniteassault-237 | Thermal Runaway | hard | base | cleared | 18/43 | 18 | 17 | 17 | 0 | 0 | 0 | 27.6 |
| infiniteassault-238 | Critical Temperature | easy | base | cleared | 6/10 | 18 | 18 | 3 | 0 | 0 | 0 | 2.0 |
| infiniteassault-238 | Critical Temperature | hard | base | cleared | 15/35 | 44 | 38 | 5 | 0 | 0 | 0 | 2.0 |
| infiniteassault-240 | Unpatched | easy | base | cleared | 7/11 | 21 | 21 | 1 | 0 | 0 | 0 | 2.0 |
| infiniteassault-240 | Unpatched | hard | base | cleared | 4/20 | 10 | 10 | 0 | 0 | 0 | 0 | n/a |
| meditative-011 | Double Header | easy | base | cleared | 7/9 | 17 | 16 | 0 | 0 | 0 | 0 | n/a |
| meditative-011 | Double Header | hard | base | cleared | 10/17 | 14 | 14 | 1 | 0 | 0 | 0 | 2.5 |
| meditative-022 | Mind Games | easy | base | cleared | 3/9 | 7 | 7 | 3 | 0 | 0 | 0 | -3.2 |
| meditative-022 | Mind Games | hard | base | cleared | 3/13 | 3 | 3 | 1 | 0 | 0 | 0 | -3.2 |
| meditative-026 | Alternate Fate | easy | base | cleared | 3/8 | 7 | 7 | 0 | 0 | 0 | 0 | n/a |
| meditative-026 | Alternate Fate | hard | base | cleared | 9/16 | 30 | 30 | 2 | 0 | 0 | 0 | 2.5 |
| meditative-053 | Prestige | easy | base | cleared | 10/17 | 15 | 1 | 0 | 0 | 0 | 0 | n/a |
| meditative-053 | Prestige | hard | base | cleared | 2/10 | 2 | 0 | 0 | 0 | 0 | 0 | n/a |
| meditative-055 | Dragon Fruit | easy | base | cleared | 7/10 | 16 | 16 | 3 | 0 | 0 | 0 | 1.9 |
| meditative-055 | Dragon Fruit | hard | base | cleared | 4/12 | 6 | 6 | 1 | 0 | 0 | 0 | 0.6 |
| meditative-060 | Eschews | easy | base | cleared | 4/7 | 7 | 7 | 3 | 0 | 0 | 0 | 2.0 |
| meditative-060 | Eschews | hard | base | cleared | 4/12 | 8 | 8 | 1 | 0 | 0 | 0 | 2.0 |
| meditative-069 | The Maestro | easy | base | cleared | 7/20 | 11 | 11 | 4 | 0 | 0 | 0 | 9.6 |
| meditative-069 | The Maestro | hard | base | cleared | 18/46 | 58 | 51 | 5 | 0 | 0 | 0 | 10.5 |
| meditative-075 | Ever Growing Tree | easy | base | cleared | 7/11 | 9 | 8 | 1 | 0 | 0 | 0 | 2.5 |
| meditative-075 | Ever Growing Tree | hard | base | cleared | 9/17 | 28 | 27 | 5 | 0 | 0 | 0 | 2.6 |
| meditative-098 | Showdown | easy | base | cleared | 4/10 | 4 | 3 | 0 | 0 | 0 | 0 | n/a |
| meditative-098 | Showdown | hard | base | cleared | 5/14 | 7 | 4 | 0 | 0 | 0 | 0 | n/a |
| meditative-099 | Paranoia | easy | base | cleared | 9/11 | 14 | 14 | 8 | 0 | 0 | 0 | 2.9 |
| meditative-099 | Paranoia | hard | base | cleared | 6/16 | 17 | 17 | 0 | 0 | 0 | 0 | n/a |
| meditativeplus-002 | NA Tech | easy | base | cleared | 2/9 | 1 | 1 | 2 | 0 | 0 | 0 | 0.7 |
| meditativeplus-002 | NA Tech | hard | base | cleared | 6/10 | 6 | 6 | 3 | 0 | 0 | 0 | 1.0 |
| meditativeplus-003 | Wondrous Wand | easy | base | cleared | 5/9 | 14 | 14 | 5 | 0 | 0 | 0 | 0.7 |
| meditativeplus-003 | Wondrous Wand | hard | base | cleared | 3/10 | 9 | 9 | 3 | 0 | 0 | 0 | 0.8 |
| meditativeplus-004 | Jlockeed's Supply Drop | easy | base | cleared | 4/10 | 8 | 8 | 1 | 0 | 0 | 0 | -1.6 |
| meditativeplus-004 | Jlockeed's Supply Drop | hard | base | cleared | 8/17 | 31 | 31 | 2 | 0 | 0 | 0 | 0.4 |
| meditativeplus-005 | Spirit Lash | easy | base | cleared | 9/12 | 19 | 19 | 6 | 0 | 0 | 0 | 1.4 |
| meditativeplus-005 | Spirit Lash | hard | base | cleared | 5/15 | 14 | 14 | 3 | 0 | 0 | 0 | 2.4 |
| meditativeplus-013 | Living Health Potion | easy | base | cleared | 21/30 | 23 | 23 | 18 | 0 | 0 | 0 | 6.6 |
| meditativeplus-013 | Living Health Potion | hard | base | cleared | 12/28 | 16 | 12 | 7 | 0 | 0 | 0 | 6.7 |
| meditativeplus-014 | Potion Pouch | easy | base | cleared | 8/12 | 23 | 23 | 5 | 0 | 0 | 0 | 0.6 |
| meditativeplus-014 | Potion Pouch | hard | base | cleared | 8/11 | 15 | 15 | 5 | 0 | 0 | 0 | 1.3 |
| meditativeplus-020 | KY's Competition | easy | base | cleared | 4/6 | 4 | 4 | 4 | 0 | 0 | 0 | -1.6 |
| meditativeplus-020 | KY's Competition | hard | base | cleared | 10/18 | 12 | 12 | 8 | 0 | 0 | 0 | -1.7 |
| meditativeplus-022 | Mountaintop Garden | easy | base | cleared | 8/13 | 25 | 25 | 5 | 0 | 0 | 0 | 2.6 |
| meditativeplus-022 | Mountaintop Garden | hard | base | cleared | 9/16 | 25 | 25 | 4 | 0 | 0 | 0 | 2.6 |
| meditativeplus-023 | Enormous Elderberry | easy | base | cleared | 7/11 | 32 | 32 | 6 | 0 | 0 | 0 | 1.0 |
| meditativeplus-023 | Enormous Elderberry | hard | base | cleared | 9/19 | 44 | 44 | 2 | 0 | 0 | 0 | 2.4 |
| stinkyscape-002 | Absolute Zero | easy | base | cleared | 6/7 | 21 | 21 | 3 | 0 | 0 | 0 | 58.5 |
| stinkyscape-002 | Absolute Zero | hard | base | cleared | 5/13 | 20 | 20 | 4 | 0 | 0 | 0 | 40.9 |
| stinkyscape-004 | Olivian Expanse | easy | base | cleared | 12/14 | 43 | 43 | 4 | 0 | 0 | 0 | -3.1 |
| stinkyscape-004 | Olivian Expanse | hard | base | cleared | 3/6 | 14 | 14 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-007 | The Greatest Heist | easy | base | cleared | 7/12 | 17 | 17 | 6 | 0 | 0 | 0 | 1.5 |
| stinkyscape-007 | The Greatest Heist | hard | base | cleared | 5/10 | 23 | 22 | 3 | 0 | 0 | 0 | 1.5 |
| stinkyscape-008 | Conjure Rosemary Triscuit | easy | base | cleared | 9/13 | 17 | 17 | 5 | 0 | 0 | 0 | -0.6 |
| stinkyscape-008 | Conjure Rosemary Triscuit | hard | base | cleared | 8/13 | 18 | 18 | 3 | 0 | 0 | 0 | 3.3 |
| stinkyscape-009 | Ticketed | easy | base | cleared | 5/7 | 4 | 4 | 3 | 0 | 0 | 0 | 7.4 |
| stinkyscape-009 | Ticketed | hard | base | cleared | 4/9 | 9 | 9 | 2 | 0 | 0 | 0 | 7.1 |
| stinkyscape-010 | Otherworldly Endurance | easy | base | cleared | 5/13 | 11 | 11 | 1 | 0 | 0 | 0 | 2.3 |
| stinkyscape-010 | Otherworldly Endurance | hard | base | cleared | 4/17 | 5 | 5 | 4 | 0 | 0 | 0 | 2.3 |
| stinkyscape-011 | Otherworldly Magic | easy | base | cleared | 4/9 | 5 | 5 | 4 | 0 | 0 | 0 | 2.3 |
| stinkyscape-011 | Otherworldly Magic | hard | base | cleared | 9/14 | 17 | 17 | 4 | 0 | 0 | 0 | 2.3 |
| stinkyscape-017 | Imprisoned Ship Smith | easy | base | cleared | 8/23 | 16 | 13 | 6 | 0 | 0 | 0 | -0.3 |
| stinkyscape-017 | Imprisoned Ship Smith | hard | base | cleared | 14/31 | 27 | 23 | 9 | 0 | 0 | 0 | 0.3 |
| stinkyscape-019 | Imprisoned Cosmality | easy | base | cleared | 10/20 | 16 | 14 | 6 | 0 | 0 | 0 | 2.9 |
| stinkyscape-019 | Imprisoned Cosmality | hard | base | cleared | 18/28 | 20 | 17 | 14 | 0 | 0 | 0 | 2.8 |
| stinkyscape-021 | Moonberry | easy | base | cleared | 4/11 | 6 | 6 | 2 | 0 | 0 | 0 | 1.6 |
| stinkyscape-021 | Moonberry | hard | base | cleared | 4/14 | 12 | 12 | 3 | 0 | 0 | 0 | 0.5 |
| stinkyscape-022 | Gatinsir | easy | base | cleared | 1/11 | 7 | 7 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-022 | Gatinsir | hard | base | cleared | 6/14 | 16 | 16 | 3 | 0 | 0 | 0 | 2.5 |
| stinkyscape-024 | Warp Disruptor II | easy | base | cleared | 2/5 | 4 | 4 | 2 | 0 | 0 | 0 | 6.0 |
| stinkyscape-024 | Warp Disruptor II | hard | base | cleared | 10/18 | 18 | 18 | 10 | 0 | 0 | 0 | 11.8 |
| stinkyscape-032 | Drone Core VI | easy | base | cleared | 11/17 | 11 | 11 | 6 | 0 | 0 | 0 | 34.3 |
| stinkyscape-032 | Drone Core VI | hard | base | cleared | 6/19 | 8 | 8 | 7 | 0 | 0 | 0 | 46.0 |
| stinkyscape-036 | Drone Core X | easy | base | cleared | 4/6 | 0 | 0 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-036 | Drone Core X | hard | base | cleared | 5/23 | 5 | 5 | 1 | 0 | 0 | 0 | 32.4 |
| stinkyscape-039 | The Narcor Project | easy | base | cleared | 10/16 | 41 | 41 | 3 | 0 | 0 | 0 | 1.2 |
| stinkyscape-039 | The Narcor Project | hard | base | cleared | 9/13 | 33 | 30 | 3 | 0 | 0 | 0 | 2.0 |
| stinkyscape-040 | Poison Capsule | easy | base | cleared | 9/24 | 22 | 15 | 3 | 0 | 0 | 0 | 3.2 |
| stinkyscape-040 | Poison Capsule | hard | base | cleared | 24/46 | 71 | 62 | 6 | 0 | 0 | 0 | 4.0 |
| stinkyscape-046 | Circadian Halfcycler | easy | base | cleared | 5/12 | 6 | 6 | 5 | 0 | 0 | 0 | 3.4 |
| stinkyscape-046 | Circadian Halfcycler | hard | base | cleared | 5/13 | 5 | 5 | 5 | 0 | 0 | 0 | 2.4 |
| stinkyscape-047 | Circadian Fullcycler | easy | base | cleared | 2/7 | 1 | 1 | 1 | 0 | 0 | 0 | 1.7 |
| stinkyscape-047 | Circadian Fullcycler | hard | base | cleared | 7/7 | 8 | 8 | 5 | 0 | 0 | 0 | 6.4 |
| stinkyscape-049 | Spacemine | easy | base | cleared | 10/16 | 11 | 11 | 11 | 0 | 0 | 0 | 7.9 |
| stinkyscape-049 | Spacemine | hard | base | cleared | 13/29 | 18 | 13 | 9 | 0 | 0 | 0 | 8.0 |
| stinkyscape-052 | Cannon v3 | easy | base | cleared | 18/27 | 35 | 32 | 14 | 0 | 0 | 0 | -0.3 |
| stinkyscape-052 | Cannon v3 | hard | base | cleared | 16/39 | 48 | 36 | 7 | 0 | 0 | 0 | 3.5 |
| stinkyscape-055 | Repeater v4 | easy | base | cleared | 12/19 | 30 | 26 | 6 | 0 | 0 | 0 | 1.7 |
| stinkyscape-055 | Repeater v4 | hard | base | cleared | 15/29 | 37 | 26 | 4 | 0 | 0 | 0 | 1.1 |
| stinkyscape-058 | Beam v5 | easy | base | cleared | 11/20 | 15 | 14 | 8 | 0 | 0 | 0 | 1.8 |
| stinkyscape-058 | Beam v5 | hard | base | cleared | 16/37 | 35 | 25 | 7 | 0 | 0 | 0 | 1.8 |
| stinkyscape-059 | Repeater v5 | easy | base | cleared | 9/18 | 21 | 21 | 6 | 0 | 0 | 0 | 2.4 |
| stinkyscape-059 | Repeater v5 | hard | base | cleared | 22/41 | 65 | 47 | 4 | 0 | 0 | 0 | 1.9 |
| stinkyscape-061 | Flak v5 | easy | base | cleared | 12/23 | 18 | 16 | 10 | 0 | 0 | 0 | 1.7 |
| stinkyscape-061 | Flak v5 | hard | base | cleared | 20/38 | 41 | 32 | 9 | 0 | 0 | 0 | 1.8 |
| stinkyscape-063 | Miner v2 | easy | base | cleared | 8/20 | 31 | 25 | 6 | 0 | 0 | 0 | -0.3 |
| stinkyscape-063 | Miner v2 | hard | base | cleared | 11/23 | 20 | 17 | 6 | 0 | 0 | 0 | -0.3 |
| stinkyscape-064 | Miner v3 | easy | base | cleared | 12/18 | 44 | 38 | 5 | 0 | 0 | 0 | 1.4 |
| stinkyscape-064 | Miner v3 | hard | base | cleared | 15/29 | 47 | 36 | 4 | 0 | 0 | 0 | 0.4 |
| stinkyscape-065 | Miner v4 | easy | base | cleared | 9/17 | 25 | 21 | 4 | 0 | 0 | 0 | 5.5 |
| stinkyscape-065 | Miner v4 | hard | base | cleared | 15/33 | 43 | 30 | 1 | 0 | 0 | 0 | 1.1 |
| stinkyscape-066 | Miner v5 | easy | base | cleared | 10/20 | 23 | 18 | 3 | 0 | 0 | 0 | 3.6 |
| stinkyscape-066 | Miner v5 | hard | base | cleared | 18/40 | 56 | 41 | 3 | 0 | 0 | 0 | 1.8 |
| stinkyscape-071 | A'Zoth | easy | base | cleared | 9/19 | 13 | 13 | 7 | 0 | 0 | 0 | 23.9 |
| stinkyscape-071 | A'Zoth | hard | base | cleared | 16/36 | 34 | 27 | 10 | 0 | 0 | 0 | 25.5 |
| stinkyscape-074 | Paragon-X | easy | base | cleared | 10/23 | 17 | 12 | 7 | 0 | 0 | 0 | -11.9 |
| stinkyscape-074 | Paragon-X | hard | base | cleared | 11/30 | 20 | 17 | 9 | 0 | 0 | 0 | -9.0 |
| stinkyscape-076 | Honey Badger | easy | base | cleared | 11/19 | 17 | 13 | 9 | 0 | 0 | 0 | 8.9 |
| stinkyscape-076 | Honey Badger | hard | base | cleared | 3/23 | 9 | 7 | 1 | 0 | 0 | 0 | 6.1 |
| stinkyscape-080 | Galaxy | easy | base | cleared | 8/17 | 29 | 17 | 1 | 0 | 0 | 0 | -10.2 |
| stinkyscape-080 | Galaxy | hard | base | cleared | 21/36 | 76 | 57 | 2 | 0 | 0 | 0 | 1.3 |
| stinkyscape-082 | Phalanx | easy | base | cleared | 5/12 | 10 | 5 | 1 | 0 | 0 | 0 | 5.5 |
| stinkyscape-082 | Phalanx | hard | base | cleared | 15/27 | 43 | 29 | 3 | 0 | 0 | 0 | 5.0 |
| stinkyscape-083 | Halo | easy | base | cleared | 4/6 | 4 | 3 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-083 | Halo | hard | base | cleared | 12/28 | 31 | 29 | 2 | 0 | 0 | 0 | 3.4 |
| stinkyscape-084 | Power Cell | easy | base | cleared | 3/8 | 25 | 25 | 1 | 0 | 0 | 0 | -0.9 |
| stinkyscape-084 | Power Cell | hard | base | cleared | 6/17 | 18 | 18 | 2 | 0 | 0 | 0 | -1.3 |
| stinkyscape-085 | Prototype-X1 | easy | base | cleared | 8/22 | 21 | 10 | 4 | 0 | 0 | 0 | 4.2 |
| stinkyscape-085 | Prototype-X1 | hard | base | cleared | 13/24 | 29 | 29 | 6 | 0 | 0 | 0 | 7.8 |
| stinkyscape-086 | Prototype-X2 | easy | base | cleared | 5/11 | 12 | 4 | 1 | 0 | 0 | 0 | 1.2 |
| stinkyscape-086 | Prototype-X2 | hard | base | cleared | 18/34 | 35 | 17 | 3 | 0 | 0 | 0 | -12.5 |
| stinkyscape-087 | Siva 1 | easy | base | cleared | 13/30 | 14 | 9 | 1 | 0 | 0 | 0 | 3.3 |
| stinkyscape-087 | Siva 1 | hard | base | cleared | 15/33 | 29 | 22 | 3 | 0 | 0 | 0 | 4.8 |
| stinkyscape-088 | Siva 2 | easy | base | cleared | 22/34 | 34 | 29 | 16 | 0 | 0 | 0 | 12.4 |
| stinkyscape-088 | Siva 2 | hard | base | cleared | 17/39 | 27 | 22 | 9 | 0 | 0 | 0 | 10.7 |
| stinkyscape-089 | Spice | easy | base | cleared | 6/13 | 11 | 11 | 5 | 0 | 0 | 0 | -0.4 |
| stinkyscape-089 | Spice | hard | base | cleared | 4/9 | 6 | 6 | 3 | 0 | 0 | 0 | -0.4 |
| stinkyscape-093 | Patcher Nanobots | easy | base | cleared | 12/22 | 8 | 7 | 11 | 0 | 0 | 0 | 10.0 |
| stinkyscape-093 | Patcher Nanobots | hard | base | cleared | 11/30 | 14 | 14 | 9 | 0 | 0 | 0 | 11.7 |
| stinkyscape-095 | Faction: Vanguard Coalition | easy | base | cleared | 4/7 | 9 | 9 | 3 | 0 | 0 | 0 | 8.8 |
| stinkyscape-095 | Faction: Vanguard Coalition | hard | base | cleared | 4/5 | 14 | 14 | 2 | 0 | 0 | 0 | 4.4 |
| stinkyscape-096 | Ocean Tsunami | easy | base | cleared | 9/17 | 17 | 8 | 3 | 0 | 0 | 0 | 21.2 |
| stinkyscape-096 | Ocean Tsunami | hard | base | cleared | 11/34 | 33 | 27 | 4 | 0 | 0 | 0 | 8.9 |
| stinkyscape-104 | Reflective Plating | easy | base | cleared | 15/21 | 34 | 29 | 11 | 0 | 0 | 0 | 6.2 |
| stinkyscape-104 | Reflective Plating | hard | base | cleared | 10/26 | 21 | 17 | 5 | 0 | 0 | 0 | 4.4 |
| stinkyscape-109 | xxFreshJack | easy | base | cleared | 13/22 | 19 | 14 | 5 | 0 | 0 | 0 | 20.1 |
| stinkyscape-109 | xxFreshJack | hard | base | cleared | 21/32 | 41 | 33 | 20 | 0 | 0 | 0 | 21.0 |
| stinkyscape-114 | Arctic Citadel | easy | base | cleared | 10/16 | 25 | 14 | 4 | 0 | 0 | 0 | 9.2 |
| stinkyscape-114 | Arctic Citadel | hard | base | cleared | 22/41 | 51 | 37 | 3 | 0 | 0 | 0 | 21.2 |
| stinkyscape-115 | Scornboi | easy | base | cleared | 15/26 | 41 | 24 | 8 | 0 | 0 | 0 | 32.5 |
| stinkyscape-115 | Scornboi | hard | base | cleared | 18/36 | 28 | 22 | 5 | 0 | 0 | 0 | 26.4 |
| stinkyscape-117 | Core System | easy | base | cleared | 6/8 | 20 | 20 | 3 | 0 | 0 | 0 | 66.3 |
| stinkyscape-117 | Core System | hard | base | cleared | 2/9 | 9 | 9 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-118 | Secure System | easy | base | cleared | 4/6 | 22 | 22 | 2 | 0 | 0 | 0 | 84.6 |
| stinkyscape-118 | Secure System | hard | base | cleared | 12/16 | 33 | 33 | 2 | 0 | 0 | 0 | 81.9 |
| stinkyscape-119 | Unsecure System | easy | base | cleared | 2/6 | 6 | 6 | 1 | 0 | 0 | 0 | 5.9 |
| stinkyscape-119 | Unsecure System | hard | base | cleared | 6/17 | 9 | 9 | 4 | 0 | 0 | 0 | 17.2 |
| stinkyscape-120 | Wild System | easy | base | cleared | 3/11 | 11 | 11 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-120 | Wild System | hard | base | cleared | 5/16 | 8 | 8 | 1 | 0 | 0 | 0 | 53.5 |
| stinkyscape-121 | System Schematic | easy | base | cleared | 12/20 | 12 | 12 | 8 | 0 | 0 | 0 | 1.0 |
| stinkyscape-121 | System Schematic | hard | base | cleared | 3/11 | 3 | 3 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-124 | Jlockeed's Shipbuilding Grant | easy | base | cleared | 11/15 | 29 | 29 | 9 | 0 | 0 | 0 | 2.8 |
| stinkyscape-124 | Jlockeed's Shipbuilding Grant | hard | base | cleared | 8/21 | 31 | 31 | 5 | 0 | 0 | 0 | 1.8 |
| stinkyscape-125 | Jlockeed's Spacerace Propaganda | easy | base | cleared | 8/9 | 18 | 18 | 7 | 0 | 0 | 0 | 1.5 |
| stinkyscape-125 | Jlockeed's Spacerace Propaganda | hard | base | cleared | 11/17 | 24 | 23 | 3 | 0 | 0 | 0 | 1.5 |
| stinkyscape-126 | Jlockeed's Permanent Procurement Lobbying | easy | base | cleared | 9/16 | 16 | 16 | 6 | 0 | 0 | 0 | 1.2 |
| stinkyscape-126 | Jlockeed's Permanent Procurement Lobbying | hard | base | cleared | 10/19 | 24 | 24 | 3 | 0 | 0 | 0 | 2.6 |
| stinkyscape-127 | Jlockeed's Siege Tank | easy | base | cleared | 7/18 | 7 | 7 | 7 | 0 | 0 | 0 | 16.0 |
| stinkyscape-127 | Jlockeed's Siege Tank | hard | base | cleared | 15/31 | 15 | 11 | 13 | 0 | 0 | 0 | 16.9 |
| stinkyscape-128 | Jlockeed's Yamato Cannon | easy | base | cleared | 10/24 | 14 | 14 | 8 | 0 | 0 | 0 | 8.5 |
| stinkyscape-128 | Jlockeed's Yamato Cannon | hard | base | cleared | 10/25 | 13 | 7 | 4 | 0 | 0 | 0 | 9.6 |
| stinkyscape-130 | Jlockeed's Starship Retrofitting Program | easy | base | cleared | 6/13 | 13 | 13 | 3 | 0 | 0 | 0 | 0.6 |
| stinkyscape-130 | Jlockeed's Starship Retrofitting Program | hard | base | cleared | 8/14 | 25 | 25 | 3 | 0 | 0 | 0 | -5.9 |
| stinkyscape-131 | Spacerace Stimulus | easy | base | cleared | 3/6 | 4 | 3 | 3 | 0 | 0 | 0 | 7.6 |
| stinkyscape-131 | Spacerace Stimulus | hard | base | cleared | 8/15 | 9 | 9 | 8 | 0 | 0 | 0 | 6.3 |
| stinkyscape-132 | Defensive Crystal | easy | base | cleared | 10/24 | 11 | 9 | 8 | 0 | 0 | 0 | 9.4 |
| stinkyscape-132 | Defensive Crystal | hard | base | cleared | 16/31 | 28 | 23 | 14 | 0 | 0 | 0 | 9.1 |
| stinkyscape-133 | Ultra Capacitor | easy | base | cleared | 8/23 | 11 | 10 | 6 | 0 | 0 | 0 | 6.0 |
| stinkyscape-133 | Ultra Capacitor | hard | base | cleared | 8/26 | 9 | 9 | 6 | 0 | 0 | 0 | 3.9 |
| stinkyscape-134 | Felinor Containment Chamber | easy | base | cleared | 6/22 | 10 | 10 | 5 | 0 | 0 | 0 | 4.3 |
| stinkyscape-134 | Felinor Containment Chamber | hard | base | cleared | 8/20 | 14 | 11 | 7 | 0 | 0 | 0 | 3.3 |
| stinkyscape-141 | Fruit Launcher | easy | base | cleared | 11/25 | 22 | 13 | 6 | 0 | 0 | 0 | 1.0 |
| stinkyscape-141 | Fruit Launcher | hard | base | cleared | 16/33 | 57 | 44 | 6 | 0 | 0 | 0 | 0.4 |
| stinkyscape-144 | Prepare For Liftoff | easy | base | cleared | 5/10 | 9 | 9 | 1 | 0 | 0 | 0 | 1.0 |
| stinkyscape-144 | Prepare For Liftoff | hard | base | cleared | 3/11 | 3 | 2 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-145 | Fortify Flesh | easy | base | cleared | 5/9 | 15 | 15 | 1 | 0 | 0 | 0 | n/a |
| stinkyscape-145 | Fortify Flesh | hard | base | cleared | 5/10 | 21 | 21 | 1 | 0 | 0 | 0 | 0.2 |
| stinkyscape-150 | Call to Chaos (Stinky Edition) | easy | base | cleared | 7/13 | 11 | 11 | 4 | 0 | 0 | 0 | 26.4 |
| stinkyscape-150 | Call to Chaos (Stinky Edition) | hard | base | cleared | 10/22 | 15 | 15 | 4 | 0 | 0 | 0 | 48.7 |
| stinkyscape-151 | Call to Chaos (Scape Edition) | easy | base | cleared | 6/10 | 20 | 20 | 2 | 0 | 0 | 0 | -11.2 |
| stinkyscape-151 | Call to Chaos (Scape Edition) | hard | base | cleared | 10/13 | 15 | 15 | 4 | 0 | 0 | 0 | 11.6 |
| stinkyscape-152 | Sanctum | easy | base | cleared | 3/7 | 3 | 3 | 1 | 0 | 0 | 0 | 5.1 |
| stinkyscape-152 | Sanctum | hard | base | cleared | 7/12 | 14 | 14 | 2 | 0 | 0 | 0 | 5.7 |
| stinkyscape-155 | Comet Crash | easy | base | cleared | 2/7 | 3 | 3 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-155 | Comet Crash | hard | base | cleared | 6/16 | 13 | 13 | 1 | 0 | 0 | 0 | -0.1 |
| stinkyscape-156 | Catalyst Comet | easy | base | cleared | 12/20 | 26 | 26 | 10 | 0 | 0 | 0 | -0.6 |
| stinkyscape-156 | Catalyst Comet | hard | base | cleared | 5/14 | 27 | 27 | 3 | 0 | 0 | 0 | 0.3 |
| stinkyscape-157 | The Gauntlet | easy | base | cleared | 5/10 | 20 | 20 | 3 | 0 | 0 | 0 | 2.4 |
| stinkyscape-157 | The Gauntlet | hard | base | cleared | 7/16 | 28 | 27 | 5 | 0 | 0 | 0 | -0.2 |
| stinkyscape-158 | Stratos Swarm | easy | base | cleared | 7/12 | 6 | 6 | 4 | 0 | 0 | 0 | 47.3 |
| stinkyscape-158 | Stratos Swarm | hard | base | cleared | 8/13 | 7 | 7 | 6 | 0 | 0 | 0 | 38.1 |
| stinkyscape-159 | Faction: ETHICS | easy | base | cleared | 4/8 | 4 | 4 | 3 | 0 | 0 | 0 | 9.8 |
| stinkyscape-159 | Faction: ETHICS | hard | base | cleared | 2/7 | 0 | 0 | 0 | 0 | 0 | 0 | n/a |
| stinkyscape-161 | Faction: Pegasus | easy | base | cleared | 5/9 | 11 | 11 | 4 | 0 | 0 | 0 | 12.4 |
| stinkyscape-161 | Faction: Pegasus | hard | base | cleared | 4/14 | 10 | 10 | 1 | 0 | 0 | 0 | 6.1 |
| stinkyscape-162 | Faction: Star-Jumper Core | easy | base | cleared | 3/11 | 16 | 16 | 3 | 0 | 0 | 0 | 4.0 |
| stinkyscape-162 | Faction: Star-Jumper Core | hard | base | cleared | 4/13 | 9 | 9 | 2 | 0 | 0 | 0 | 5.4 |
| stinkyscape-163 | Swear Loyalities | easy | base | cleared | 3/8 | 2 | 2 | 3 | 0 | 0 | 0 | 0.6 |
| stinkyscape-163 | Swear Loyalities | hard | base | cleared | 7/13 | 5 | 5 | 5 | 0 | 0 | 0 | 0.4 |
| stinkyscape-164 | The Syndicate | easy | base | cleared | 7/12 | 8 | 8 | 1 | 0 | 0 | 0 | 0.5 |
| stinkyscape-164 | The Syndicate | hard | base | cleared | 8/17 | 8 | 8 | 3 | 0 | 0 | 0 | 0.5 |
| stinkyscape-165 | The Trade Union | easy | base | cleared | 6/9 | 7 | 7 | 4 | 0 | 0 | 0 | -1.8 |
| stinkyscape-165 | The Trade Union | hard | base | cleared | 5/14 | 4 | 4 | 3 | 0 | 0 | 0 | 2.5 |
| stinkyscape-166 | Scanning Probe Mk1 | easy | base | cleared | 4/13 | 11 | 11 | 2 | 0 | 0 | 0 | -1.1 |
| stinkyscape-166 | Scanning Probe Mk1 | hard | base | cleared | 6/12 | 19 | 19 | 4 | 0 | 0 | 0 | -0.6 |
| stinkyscape-167 | Scanning Probe Mk2 | easy | base | cleared | 3/6 | 6 | 6 | 2 | 0 | 0 | 0 | -1.4 |
| stinkyscape-167 | Scanning Probe Mk2 | hard | base | cleared | 6/19 | 19 | 19 | 1 | 0 | 0 | 0 | 0.6 |
| stinkyscape-171 | Large Asteroid | easy | base | cleared | 12/20 | 42 | 33 | 4 | 0 | 0 | 0 | 6.1 |
| stinkyscape-171 | Large Asteroid | hard | base | cleared | 14/30 | 42 | 34 | 2 | 0 | 0 | 0 | 6.1 |
| stinkyscape-172 | Huge Asteroid | easy | base | cleared | 15/28 | 26 | 20 | 5 | 0 | 0 | 0 | 7.3 |
| stinkyscape-172 | Huge Asteroid | hard | base | cleared | 16/33 | 46 | 36 | 6 | 0 | 0 | 0 | 7.0 |
| stinkyscape-173 | Abandoned Spice Tower | easy | base | cleared | 6/21 | 7 | 6 | 5 | 0 | 0 | 0 | 6.4 |
| stinkyscape-173 | Abandoned Spice Tower | hard | base | cleared | 15/34 | 35 | 25 | 9 | 0 | 0 | 0 | 6.4 |
| stinkyscape-174 | Aerelret | easy | base | cleared | 6/13 | 10 | 9 | 2 | 0 | 0 | 0 | 2.5 |
| stinkyscape-174 | Aerelret | hard | base | cleared | 5/20 | 14 | 14 | 2 | 0 | 0 | 0 | 3.8 |
| stinkyscape-175 | Frost Module | easy | base | cleared | 14/22 | 27 | 17 | 12 | 0 | 0 | 0 | 2.8 |
| stinkyscape-175 | Frost Module | hard | base | cleared | 10/22 | 14 | 12 | 7 | 0 | 0 | 0 | 3.3 |
| stinkyscapeplus-001 | Starfruit | easy | base | cleared | 11/19 | 28 | 28 | 8 | 0 | 0 | 0 | 3.7 |
| stinkyscapeplus-001 | Starfruit | hard | base | cleared | 5/14 | 12 | 12 | 2 | 0 | 0 | 0 | 3.7 |
| stinkyscapeplus-004 | Profe Yates | easy | base | cleared | 16/33 | 39 | 31 | 14 | 0 | 0 | 0 | 8.0 |
| stinkyscapeplus-004 | Profe Yates | hard | base | cleared | 22/47 | 47 | 32 | 16 | 0 | 0 | 0 | 10.8 |
| stinkyscapeplus-005 | Dingleberry Droat | easy | base | cleared | 19/32 | 53 | 40 | 16 | 0 | 0 | 0 | 5.9 |
| stinkyscapeplus-005 | Dingleberry Droat | hard | base | cleared | 16/34 | 36 | 26 | 3 | 0 | 0 | 0 | 5.0 |
| stinkyscapeplus-008 | Drone Assembler v3 | easy | base | cleared | 9/22 | 8 | 6 | 5 | 0 | 0 | 0 | 0.4 |
| stinkyscapeplus-008 | Drone Assembler v3 | hard | base | cleared | 15/30 | 35 | 31 | 9 | 0 | 0 | 0 | 0.6 |
| stinkyscapeplus-009 | Drone Assembler v4 | easy | base | cleared | 18/33 | 30 | 25 | 13 | 0 | 0 | 0 | 1.1 |
| stinkyscapeplus-009 | Drone Assembler v4 | hard | base | cleared | 14/39 | 27 | 23 | 10 | 0 | 0 | 0 | 2.0 |
| stinkyscapeplus-013 | Shield Repairer | easy | base | cleared | 6/18 | 15 | 13 | 2 | 0 | 0 | 0 | 2.2 |
| stinkyscapeplus-013 | Shield Repairer | hard | base | cleared | 15/34 | 30 | 23 | 6 | 0 | 0 | 0 | 4.2 |
| stinkyscapeplus-015 | Hull Repairer mk2 | easy | base | cleared | 15/20 | 32 | 23 | 7 | 0 | 0 | 0 | 4.1 |
| stinkyscapeplus-015 | Hull Repairer mk2 | hard | base | cleared | 21/33 | 50 | 34 | 11 | 0 | 0 | 0 | 5.3 |
| stinkyscapeplus-023 | GIGA-ify | easy | base | cleared | 4/9 | 9 | 9 | 2 | 0 | 0 | 0 | 0.4 |
| stinkyscapeplus-023 | GIGA-ify | hard | base | cleared | 10/22 | 39 | 39 | 4 | 0 | 0 | 0 | 0.8 |
| stinkyscapeplus-024 | Forced Propagation | easy | base | cleared | 4/7 | 20 | 20 | 2 | 0 | 0 | 0 | -0.6 |
| stinkyscapeplus-024 | Forced Propagation | hard | base | cleared | 9/18 | 29 | 29 | 4 | 0 | 0 | 0 | -1.3 |
| stinkyscapeplus-026 | Jlockeed’s Forever War | easy | base | cleared | 7/12 | 5 | 5 | 7 | 0 | 0 | 0 | 1.3 |
| stinkyscapeplus-026 | Jlockeed’s Forever War | hard | base | cleared | 9/18 | 19 | 19 | 9 | 0 | 0 | 0 | 1.5 |
| stinkyscapeplus-030 | Yeah I’m a War Hero | easy | base | cleared | 8/12 | 20 | 20 | 4 | 0 | 0 | 0 | 1.0 |
| stinkyscapeplus-030 | Yeah I’m a War Hero | hard | base | cleared | 3/11 | 7 | 7 | 2 | 0 | 0 | 0 | 1.0 |
| stinkyscapeplus-032 | Z-Patch | easy | base | cleared | 7/7 | 12 | 12 | 4 | 0 | 0 | 0 | 2.5 |
| stinkyscapeplus-032 | Z-Patch | hard | base | cleared | 5/14 | 10 | 10 | 1 | 0 | 0 | 0 | 16.6 |
| stinkyscapeplus-033 | True Conservative | easy | base | cleared | 10/19 | 15 | 11 | 2 | 0 | 0 | 0 | 17.0 |
| stinkyscapeplus-033 | True Conservative | hard | base | cleared | 13/34 | 32 | 24 | 2 | 0 | 0 | 0 | 20.2 |
| stinkyscapeplus-038 | Arc | easy | base | cleared | 14/22 | 25 | 23 | 13 | 0 | 0 | 0 | 2.4 |
| stinkyscapeplus-038 | Arc | hard | base | cleared | 14/33 | 38 | 27 | 6 | 0 | 0 | 0 | 2.4 |
| stinkyscapeplus-040 | Mender | easy | base | cleared | 4/12 | 6 | 6 | 5 | 0 | 0 | 0 | 0.2 |
| stinkyscapeplus-040 | Mender | hard | base | cleared | 3/9 | 3 | 3 | 3 | 0 | 0 | 0 | 0.5 |
| stinkyscapeplus-041 | Mend Projector | easy | base | cleared | 3/12 | 9 | 9 | 2 | 0 | 0 | 0 | 2.0 |
| stinkyscapeplus-041 | Mend Projector | hard | base | cleared | 11/24 | 32 | 32 | 6 | 0 | 0 | 0 | 2.0 |
| stinkyscapeplus-043 | Segment | easy | base | cleared | 8/14 | 24 | 24 | 5 | 0 | 0 | 0 | 0.8 |
| stinkyscapeplus-043 | Segment | hard | base | cleared | 3/11 | 15 | 15 | 1 | 0 | 0 | 0 | 1.5 |
| stinkyscapeplus-046 | Phase Fabric | easy | base | cleared | 10/18 | 20 | 20 | 9 | 0 | 0 | 0 | 3.6 |
| stinkyscapeplus-046 | Phase Fabric | hard | base | cleared | 5/14 | 10 | 10 | 5 | 0 | 0 | 0 | 3.6 |
| stinkyscapeplus-049 | Metaglass | easy | base | cleared | 7/18 | 36 | 36 | 2 | 0 | 0 | 0 | 0.7 |
| stinkyscapeplus-049 | Metaglass | hard | base | cleared | 7/19 | 20 | 20 | 2 | 0 | 0 | 0 | -0.8 |
| stinkyscapeplus-050 | Graphite | easy | base | cleared | 14/20 | 54 | 54 | 10 | 0 | 0 | 0 | -0.2 |
| stinkyscapeplus-050 | Graphite | hard | base | cleared | 6/13 | 16 | 16 | 4 | 0 | 0 | 0 | 0.3 |
| stinkyscapeplus-052 | Airblast Drill | easy | base | cleared | 3/7 | 4 | 2 | 1 | 0 | 0 | 0 | 24.9 |
| stinkyscapeplus-052 | Airblast Drill | hard | base | cleared | 6/13 | 14 | 9 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-053 | Laser Drill | easy | base | cleared | 3/5 | 11 | 8 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-053 | Laser Drill | hard | base | cleared | 11/27 | 35 | 16 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-054 | Pneumatic Drill | easy | base | cleared | 8/13 | 38 | 30 | 4 | 0 | 0 | 0 | 3.0 |
| stinkyscapeplus-054 | Pneumatic Drill | hard | base | cleared | 9/14 | 31 | 31 | 4 | 0 | 0 | 0 | 1.4 |
| stinkyscapeplus-056 | Impact Reactor | easy | base | cleared | 6/15 | 23 | 11 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-056 | Impact Reactor | hard | base | cleared | 6/14 | 30 | 13 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-057 | Phase Weaver | easy | base | cleared | 4/7 | 3 | 3 | 4 | 0 | 0 | 0 | 0.5 |
| stinkyscapeplus-057 | Phase Weaver | hard | base | cleared | 6/21 | 7 | 6 | 5 | 0 | 0 | 0 | 0.5 |
| stinkyscapeplus-060 | Radi | easy | base | cleared | 9/20 | 18 | 15 | 6 | 0 | 0 | 0 | 1.0 |
| stinkyscapeplus-060 | Radi | hard | base | cleared | 17/34 | 45 | 41 | 6 | 0 | 0 | 0 | 3.2 |
| stinkyscapeplus-063 | Overdrive Dome | easy | base | cleared | 2/5 | 22 | 18 | 1 | 0 | 0 | 0 | 2.5 |
| stinkyscapeplus-063 | Overdrive Dome | hard | base | cleared | 9/18 | 26 | 25 | 1 | 0 | 0 | 0 | 2.5 |
| stinkyscapeplus-068 | Shadiris Visions | easy | base | cleared | 6/8 | 14 | 14 | 4 | 0 | 0 | 0 | -0.7 |
| stinkyscapeplus-068 | Shadiris Visions | hard | base | cleared | 7/14 | 19 | 19 | 5 | 0 | 0 | 0 | -0.2 |
| stinkyscapeplus-069 | Exotic Jellybean | easy | base | cleared | 8/18 | 45 | 45 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-069 | Exotic Jellybean | hard | base | cleared | 6/16 | 17 | 17 | 1 | 0 | 0 | 0 | 0.4 |
| stinkyscapeplus-072 | Foreshadow | easy | base | cleared | 8/13 | 25 | 0 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-072 | Foreshadow | hard | base | cleared | 14/34 | 52 | 15 | 2 | 0 | 0 | 0 | 4.4 |
| stinkyscapeplus-073 | Ripple | easy | base | cleared | 11/28 | 13 | 13 | 12 | 0 | 0 | 0 | 13.1 |
| stinkyscapeplus-073 | Ripple | hard | base | cleared | 16/25 | 26 | 26 | 15 | 0 | 0 | 0 | 13.9 |
| stinkyscapeplus-077 | Black Hole | easy | base | cleared | 6/10 | 10 | 10 | 2 | 0 | 0 | 0 | 12.7 |
| stinkyscapeplus-077 | Black Hole | hard | base | cleared | 9/18 | 28 | 28 | 1 | 0 | 0 | 0 | -2.3 |
| stinkyscapeplus-079 | A Glimpse into the Endless Sea | easy | base | cleared | 2/6 | 0 | 0 | 2 | 0 | 0 | 0 | -15.2 |
| stinkyscapeplus-079 | A Glimpse into the Endless Sea | hard | base | cleared | 11/23 | 10 | 10 | 7 | 0 | 0 | 0 | 12.1 |
| stinkyscapeplus-085 | Body Snatch | easy | base | cleared | 5/11 | 12 | 12 | 3 | 0 | 0 | 0 | 9.9 |
| stinkyscapeplus-085 | Body Snatch | hard | base | cleared | 3/12 | 14 | 14 | 3 | 0 | 0 | 0 | 5.7 |
| stinkyscapeplus-087 | Frontier Flora | easy | base | cleared | 19/26 | 52 | 49 | 11 | 0 | 0 | 0 | 0.1 |
| stinkyscapeplus-087 | Frontier Flora | hard | base | cleared | 4/13 | 8 | 8 | 1 | 0 | 0 | 0 | 1.0 |
| stinkyscapeplus-098 | Doomed Ship | easy | base | cleared | 7/15 | 15 | 15 | 2 | 0 | 0 | 0 | 2.2 |
| stinkyscapeplus-098 | Doomed Ship | hard | base | cleared | 10/21 | 35 | 35 | 5 | 0 | 0 | 0 | 1.8 |
| stinkyscapeplus-103 | Transcend | easy | base | cleared | 3/5 | 8 | 8 | 4 | 0 | 0 | 0 | 28.2 |
| stinkyscapeplus-103 | Transcend | hard | base | cleared | 9/17 | 19 | 19 | 7 | 0 | 0 | 0 | 25.7 |
| stinkyscapeplus-106 | Spectre | easy | base | cleared | 13/19 | 15 | 8 | 7 | 0 | 0 | 0 | 24.2 |
| stinkyscapeplus-106 | Spectre | hard | base | cleared | 13/32 | 26 | 15 | 7 | 0 | 0 | 0 | 21.6 |
| stinkyscapeplus-109 | Lancer | easy | base | cleared | 15/27 | 42 | 41 | 15 | 0 | 0 | 0 | 4.0 |
| stinkyscapeplus-109 | Lancer | hard | base | cleared | 12/28 | 29 | 29 | 7 | 0 | 0 | 0 | 4.1 |
| stinkyscapeplus-111 | Mace | easy | base | cleared | 14/28 | 20 | 17 | 9 | 0 | 0 | 0 | 6.3 |
| stinkyscapeplus-111 | Mace | hard | base | cleared | 8/19 | 12 | 11 | 7 | 0 | 0 | 0 | 9.2 |
| stinkyscapeplus-113 | Scepter | easy | base | cleared | 14/21 | 38 | 27 | 9 | 0 | 0 | 0 | 13.6 |
| stinkyscapeplus-113 | Scepter | hard | base | cleared | 16/33 | 38 | 26 | 13 | 0 | 0 | 0 | 13.5 |
| stinkyscapeplus-119 | Eclipse | easy | base | cleared | 10/17 | 30 | 8 | 3 | 0 | 0 | 0 | 25.5 |
| stinkyscapeplus-119 | Eclipse | hard | base | cleared | 11/24 | 25 | 15 | 3 | 0 | 0 | 0 | 19.5 |
| stinkyscapeplus-123 | Oct | easy | base | cleared | 11/24 | 42 | 21 | 3 | 0 | 0 | 0 | 4.4 |
| stinkyscapeplus-123 | Oct | hard | base | cleared | 17/34 | 36 | 29 | 5 | 0 | 0 | 0 | 2.7 |
| stinkyscapeplus-125 | Omura | easy | base | cleared | 9/24 | 16 | 7 | 7 | 0 | 0 | 0 | 20.4 |
| stinkyscapeplus-125 | Omura | hard | base | cleared | 18/38 | 29 | 17 | 12 | 0 | 0 | 0 | 17.3 |
| stinkyscapeplus-126 | Vela | easy | base | cleared | 10/21 | 44 | 35 | 1 | 0 | 0 | 0 | 18.4 |
| stinkyscapeplus-126 | Vela | hard | base | cleared | 11/24 | 27 | 23 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-129 | Multiplicative Reconstructor | easy | base | cleared | 4/10 | 6 | 6 | 3 | 0 | 0 | 0 | 1.0 |
| stinkyscapeplus-129 | Multiplicative Reconstructor | hard | base | cleared | 6/17 | 21 | 21 | 4 | 0 | 0 | 0 | 1.3 |
| stinkyscapeplus-130 | Exponential Reconstructor | easy | base | cleared | 4/10 | 15 | 12 | 1 | 0 | 0 | 0 | 1.0 |
| stinkyscapeplus-130 | Exponential Reconstructor | hard | base | cleared | 11/20 | 32 | 30 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-131 | Tetrative Reconstructor | easy | base | cleared | 3/7 | 12 | 9 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-131 | Tetrative Reconstructor | hard | base | cleared | 6/16 | 25 | 13 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-133 | Great Paperclip Theory III | easy | base | cleared | 2/8 | 6 | 6 | 1 | 0 | 0 | 0 | -1.2 |
| stinkyscapeplus-133 | Great Paperclip Theory III | hard | base | cleared | 7/15 | 24 | 24 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-136 | Crawler Assembly Line | easy | base | cleared | 6/12 | 16 | 16 | 3 | 0 | 0 | 0 | 2.0 |
| stinkyscapeplus-136 | Crawler Assembly Line | hard | base | cleared | 6/18 | 11 | 11 | 4 | 0 | 0 | 0 | 2.8 |
| stinkyscapeplus-138 | Delta Definition | easy | base | cleared | 3/8 | 5 | 5 | 2 | 0 | 0 | 0 | 4.1 |
| stinkyscapeplus-138 | Delta Definition | hard | base | cleared | 7/15 | 21 | 21 | 3 | 0 | 0 | 0 | 1.6 |
| stinkyscapeplus-140 | Automa Reintegration Plan | easy | base | cleared | 2/8 | 2 | 2 | 2 | 0 | 0 | 0 | -1.3 |
| stinkyscapeplus-140 | Automa Reintegration Plan | hard | base | cleared | 4/16 | 10 | 10 | 2 | 0 | 0 | 0 | -1.3 |
| stinkyscapeplus-142 | Call to Chaos (Mindustrial-Swarm Edition) | easy | base | cleared | 9/14 | 17 | 17 | 5 | 0 | 0 | 0 | 14.7 |
| stinkyscapeplus-142 | Call to Chaos (Mindustrial-Swarm Edition) | hard | base | cleared | 11/18 | 24 | 24 | 9 | 0 | 0 | 0 | 4.9 |
| stinkyscapeplus-143 | Fire Beam v3 | easy | base | cleared | 17/26 | 43 | 35 | 7 | 0 | 0 | 0 | 0.4 |
| stinkyscapeplus-143 | Fire Beam v3 | hard | base | cleared | 12/26 | 47 | 34 | 3 | 0 | 0 | 0 | 0.2 |
| stinkyscapeplus-144 | Fire Beam v4 | easy | base | cleared | 7/15 | 22 | 21 | 6 | 0 | 0 | 0 | 1.1 |
| stinkyscapeplus-144 | Fire Beam v4 | hard | base | cleared | 18/37 | 36 | 29 | 4 | 0 | 0 | 0 | 1.1 |
| stinkyscapeplus-145 | Fire Beam v5 | easy | base | cleared | 9/13 | 14 | 12 | 4 | 0 | 0 | 0 | 3.0 |
| stinkyscapeplus-145 | Fire Beam v5 | hard | base | cleared | 14/28 | 42 | 26 | 4 | 0 | 0 | 0 | 1.8 |
| stinkyscapeplus-146 | Scourge Mortar v3 | easy | base | cleared | 20/31 | 36 | 33 | 11 | 0 | 0 | 0 | 0.4 |
| stinkyscapeplus-146 | Scourge Mortar v3 | hard | base | cleared | 12/32 | 35 | 32 | 8 | 0 | 0 | 0 | 0.3 |
| stinkyscapeplus-147 | Scourge Mortar v4 | easy | base | cleared | 18/27 | 34 | 33 | 14 | 0 | 0 | 0 | 1.1 |
| stinkyscapeplus-147 | Scourge Mortar v4 | hard | base | cleared | 15/35 | 61 | 45 | 1 | 0 | 0 | 0 | 1.1 |
| stinkyscapeplus-148 | Scourge Mortar v5 | easy | base | cleared | 15/25 | 34 | 23 | 6 | 0 | 0 | 0 | 1.8 |
| stinkyscapeplus-148 | Scourge Mortar v5 | hard | base | cleared | 13/34 | 27 | 16 | 1 | 0 | 0 | 0 | 1.8 |
| stinkyscapeplus-150 | Scorching Rounds | easy | base | cleared | 9/17 | 11 | 9 | 7 | 0 | 0 | 0 | 4.9 |
| stinkyscapeplus-150 | Scorching Rounds | hard | base | cleared | 21/37 | 54 | 32 | 8 | 0 | 0 | 0 | 4.7 |
| stinkyscapeplus-151 | Volcanic Rounds | easy | base | cleared | 12/21 | 25 | 25 | 3 | 0 | 0 | 0 | 4.6 |
| stinkyscapeplus-151 | Volcanic Rounds | hard | base | cleared | 9/20 | 29 | 14 | 0 | 0 | 0 | 0 | n/a |
| stinkyscapeplus-153 | Noxious Rounds | easy | base | cleared | 17/26 | 52 | 46 | 6 | 0 | 0 | 0 | 8.8 |
| stinkyscapeplus-153 | Noxious Rounds | hard | base | cleared | 16/34 | 78 | 55 | 1 | 0 | 0 | 0 | 2.7 |
| stinkyscapeplus-155 | The Far Vagabond | easy | base | cleared | 4/9 | 6 | 6 | 1 | 0 | 0 | 0 | 4.7 |
| stinkyscapeplus-155 | The Far Vagabond | hard | base | cleared | 5/10 | 18 | 18 | 2 | 0 | 0 | 0 | 3.2 |
| stinkyscapeplus-156 | Horizon Breaker | easy | base | cleared | 10/18 | 19 | 16 | 4 | 0 | 0 | 0 | 10.3 |
| stinkyscapeplus-156 | Horizon Breaker | hard | base | cleared | 11/31 | 20 | 19 | 4 | 0 | 0 | 0 | 20.3 |
| stinkyscapeplus-160 | Icarus | easy | base | cleared | 11/21 | 29 | 17 | 1 | 0 | 0 | 0 | 2.5 |
| stinkyscapeplus-160 | Icarus | hard | base | cleared | 14/27 | 44 | 27 | 1 | 0 | 0 | 0 | 10.3 |
| stinkyscapeplus-165 | Shadiris | easy | base | cleared | 16/22 | 18 | 15 | 12 | 0 | 0 | 0 | 16.8 |
| stinkyscapeplus-165 | Shadiris | hard | base | cleared | 19/34 | 38 | 25 | 8 | 0 | 0 | 0 | 16.3 |
| stinkyscapeplus-166 | Mindustrial Revolution | easy | base | cleared | 5/11 | 12 | 12 | 3 | 0 | 0 | 0 | -1.2 |
| stinkyscapeplus-166 | Mindustrial Revolution | hard | base | cleared | 11/20 | 30 | 30 | 6 | 0 | 0 | 0 | 2.7 |
| traininggrounds-007 | Chosen by the Moon | easy | base | cleared | 8/17 | 7 | 6 | 8 | 0 | 0 | 0 | 12.0 |
| traininggrounds-007 | Chosen by the Moon | hard | base | cleared | 11/25 | 11 | 11 | 9 | 0 | 0 | 0 | 11.8 |
| traininggrounds-012 | Fortify | easy | base | cleared | 3/11 | 3 | 3 | 3 | 0 | 0 | 0 | 55.1 |
| traininggrounds-012 | Fortify | hard | base | cleared | 5/15 | 5 | 5 | 6 | 0 | 0 | 0 | 27.3 |
| traininggrounds-013 | Explosive Runes | easy | base | cleared | 9/17 | 17 | 17 | 8 | 0 | 0 | 0 | 0.8 |
| traininggrounds-013 | Explosive Runes | hard | base | cleared | 10/15 | 15 | 15 | 6 | 0 | 0 | 0 | 1.3 |
| traininggrounds-016 | Celestial Caller | easy | base | cleared | 10/18 | 12 | 8 | 8 | 0 | 0 | 0 | 50.1 |
| traininggrounds-016 | Celestial Caller | hard | base | cleared | 17/35 | 21 | 14 | 10 | 0 | 0 | 0 | 31.9 |
| traininggrounds-017 | Flaming Felinor | easy | base | cleared | 16/30 | 23 | 22 | 14 | 0 | 0 | 0 | 24.8 |
| traininggrounds-017 | Flaming Felinor | hard | base | cleared | 11/26 | 26 | 22 | 6 | 0 | 0 | 0 | 14.5 |
| traininggrounds-019 | Extra Rations | easy | base | cleared | 4/9 | 8 | 8 | 4 | 0 | 0 | 0 | 0.5 |
| traininggrounds-019 | Extra Rations | hard | base | cleared | 6/19 | 7 | 7 | 6 | 0 | 0 | 0 | 1.0 |
| traininggrounds-020 | Recruitment Posters | easy | base | cleared | 2/4 | 9 | 9 | 1 | 0 | 0 | 0 | 3.6 |
| traininggrounds-020 | Recruitment Posters | hard | base | cleared | 2/9 | 7 | 7 | 1 | 0 | 0 | 0 | 1.5 |
| traininggrounds-031 | Level Up! | easy | base | cleared | 7/14 | 36 | 35 | 4 | 0 | 0 | 0 | 41.1 |
| traininggrounds-031 | Level Up! | hard | base | cleared | 10/22 | 29 | 29 | 4 | 0 | 0 | 0 | 2.5 |
| traininggrounds-035 | Corpse Recycler | easy | base | cleared | 5/6 | 16 | 15 | 3 | 0 | 0 | 0 | 5.7 |
| traininggrounds-035 | Corpse Recycler | hard | base | cleared | 7/14 | 19 | 19 | 3 | 0 | 0 | 0 | 2.0 |
| traininggrounds-036 | Training Dummy | easy | base | cleared | 15/33 | 37 | 31 | 11 | 0 | 0 | 0 | 2.7 |
| traininggrounds-036 | Training Dummy | hard | base | cleared | 8/24 | 30 | 22 | 3 | 0 | 0 | 0 | 1.7 |
| traininggrounds-037 | Seance | easy | base | cleared | 4/7 | 14 | 14 | 1 | 0 | 0 | 0 | 6.7 |
| traininggrounds-037 | Seance | hard | base | cleared | 8/16 | 26 | 26 | 5 | 0 | 0 | 0 | 2.4 |
| traininggrounds-038 | Tutorial Master | easy | base | cleared | 10/14 | 7 | 6 | 6 | 0 | 0 | 0 | 26.7 |
| traininggrounds-038 | Tutorial Master | hard | base | cleared | 12/28 | 14 | 11 | 9 | 0 | 0 | 0 | 26.3 |
| traininggrounds-046 | Legacy: Hit Job | easy | base | cleared | 8/11 | 18 | 18 | 5 | 0 | 0 | 0 | 15.1 |
| traininggrounds-046 | Legacy: Hit Job | hard | base | cleared | 11/22 | 17 | 17 | 7 | 0 | 0 | 0 | 7.5 |
| traininggrounds-049 | Legacy: Reoccurring Dream | easy | base | cleared | 9/20 | 31 | 31 | 11 | 0 | 0 | 0 | 0.4 |
| traininggrounds-049 | Legacy: Reoccurring Dream | hard | base | cleared | 6/13 | 33 | 33 | 16 | 0 | 0 | 0 | 3.2 |
| traininggrounds-050 | Legacy: Glowy Jelly Bean | easy | base | cleared | 8/16 | 25 | 25 | 3 | 0 | 0 | 0 | 6.1 |
| traininggrounds-050 | Legacy: Glowy Jelly Bean | hard | base | cleared | 5/10 | 11 | 11 | 3 | 0 | 0 | 0 | 15.8 |
| traininggrounds-053 | Legacy: Prejudiced Postdoc | easy | base | cleared | 16/31 | 38 | 35 | 13 | 0 | 0 | 0 | 16.5 |
| traininggrounds-053 | Legacy: Prejudiced Postdoc | hard | base | cleared | 9/29 | 20 | 17 | 7 | 0 | 0 | 0 | 16.5 |
| traininggrounds-055 | Legacy: Reminisce | easy | base | cleared | 9/15 | 28 | 28 | 7 | 0 | 0 | 0 | -1.4 |
| traininggrounds-055 | Legacy: Reminisce | hard | base | cleared | 8/14 | 21 | 21 | 5 | 0 | 0 | 0 | -1.2 |
| traininggrounds-060 | Legacy: Rush Token Farm | easy | base | cleared | 8/12 | 17 | 17 | 10 | 0 | 0 | 0 | 1.2 |
| traininggrounds-060 | Legacy: Rush Token Farm | hard | base | cleared | 7/13 | 9 | 9 | 6 | 0 | 0 | 0 | 1.2 |

### Suspects (an error or timeout in a pass-2 game that also dealt this at-risk card as filler)

None.

### Unswept (never affordable at any tier: no evidence either way)

None.

### For crates/ai/src/shadow_ban.rs

Header line:

// Sweep of record: 2026-11-26 (UTC), `cargo jackioh sweep`, pass 1 over 8 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 8 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), each card also on its Radiant face (`sweep:<tier>:<id>:radiant:<n>`, `sweep2:<tier>:<id>:radiant:<n>`), budget AI_GATE_BUDGET {"nodes":1200,"lethalNodes":300,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.

SHADOW_BAN entries:

```rust
    ("infiniteassault-048", "neverPlayed: hard (Radiant): affordable in hand on 44 turns and playable on 44 over 24 pass-2 games, never played"),
    ("infiniteassault-149", "timeout: hard: 1 decision(s) over 2000 ms or game(s) past 600 actions"),
    ("infiniteassault-211", "neverPlayed: hard (Radiant): affordable in hand on 43 turns and playable on 41 over 24 pass-2 games, never played"),
```

SHADOW_WATCH entries:

```rust
    ("infiniteassault-029", "at risk: easy: pass 1 affordable on 9 turns and playable on 9 over 8 games, played 1 time(s), mean evaluate change 1.0"),
    ("infiniteassault-035", "at risk: easy (Radiant): pass 1 affordable on 11 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change 13.8"),
    ("infiniteassault-072", "at risk: easy (Radiant): pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change -2.7"),
    ("infiniteassault-123", "at risk: easy: pass 1 affordable on 7 turns and playable on 7 over 8 games, never played; easy (Radiant): pass 1 affordable on 24 turns and playable on 24 over 8 games, never played; hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, never played; hard (Radiant): pass 1 affordable on 7 turns and playable on 7 over 8 games, never played"),
    ("infiniteassault-161", "at risk: hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 6.8"),
```

Cleared ids:

```rust
```
