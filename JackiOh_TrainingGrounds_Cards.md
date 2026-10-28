The designer's list for the Training Grounds set, as uploaded on 2026-10-28 (as `JackiOh_v5.md`), verbatim (typos, numbering and formatting as the designer wrote them, Markdown escapes included; `~~~` separates a base face from its Radiant face). With it the owner asked that the tutorial be remade from these cards, that it be impossible to lose, that it let a player discover the mechanics for themselves, and that the set also ship to the normal game. `docs/training-grounds-set.md` reads it and plans the issues; SPEC (§7, §8) and `crates/cards/catalog.json` will be the implemented reading, not this file.

```text
All cards are training grounds;  
1-15 Common  
16-27 Rare  
28-36 Epic  
37-39 Legendary  
40 Mythic  
*{Legendary & Mythic cards in training grounds DO NOT SHOW UP until the last game}*

**(1) Spikey Steven, Unit, Human, \#1**  
5/2  
\~\~\~  
15/6

**(1) Personal Trainer, Unit, Human, \#2**  
3/2  
**Cry:** Give a different ally **Human** \+2/+2   
\~\~\~  
6/4  
**Cry:** Give a friendly **Human** \+4/+4

**(1) Group leader, Unit, Human, \#3**  
4/1  
**Aura:** Your other units have \+1 attack  
\~\~\~  
8/2  
**Aura:** Your other units have \+2 attack

**(2) Boulder Fist Ogre, Unit, \#4**  
**6/7**  
Rush  
**\~\~\~**  
**18/21**  
Rush

**(2) Midlane Menace, Unit, Human, \#5**  
3/3  
**Cry:** if this was placed in the Middle lane gain \+5/+5 and **Rush**  
\~\~\~  
6/6  
**Cry:** if this was placed in the Middle lane gain \+10/+10 and **Rush**

**(1) Chosen by the Sun, Unit, \#6**  
**3/3**  
**Divine Shield**  
\~\~\~  
**9/9**  
**Divine Shield**

**(1) Chosen by the Moon, Unit, \#7**  
**3/3**  
**Taunt**  
**Armor 3**  
**\~\~\~**  
**6/6**  
**Taunt**  
**Armor 6**

**(3) Clobberer, Unit, Human, \#8**  
14/7  
\~\~\~  
42/14

**(4) Ravager, Unit, Human, \#9**  
7/7  
**Rush**  
On kill gain \+5/+5  
\~\~\~  
14/17  
**Rush**  
On kill gain \+10/+10

**(1) Fan of Knives, Spell, \#10**  
Deal 1 damage to all enemies**,** draw 1  
**Pierce**  
\~\~\~  
Deal 2 damage to all enemies**,** draw 1  
**Pierce**  
**Echo 1**

**(2) Thwomp, Spell, \#11**  
Deal 10 damage to a unit  
\~\~\~  
Deal 100 damage to a unit

**(1) Fortify, Spell, \#12**  
Give a unit \+3 health, \+1 Armor, and **Taunt**  
\~\~\~  
Give a unit \+6 health, \+2 Armor, **Taunt** and **Divine Shield**

**(1) Explosive Runes, Trap, \#13**  
When a unit attacks your hero, deal 3 damage to all enemies  
*{Resolves before the attack resolves, such that if the unit attacked your hero with \<=3 life an no armor it would die and your hero wouldn’t take damage}*  
\~\~\~  
When a unit attacks your hero, deal 6 damage to all enemies. **Pierce**

**(1) Explosive Trap, Trap, \#14**  
After your opponents plays a unit, deal 6 damage to it. **Trample**  
\~\~\~  
After your opponents plays a unit, deal 12 damage to it. **Trample, Pierce**

**(2) In a hurry, Spell, \#15**  
Summon 3 rush tokens that die at the end of this turn *{give them brittle 1}*  
\~\~\~  
Summon 3 **Radiant** rush tokens that die at the end of this turn *{give them brittle 1}*

**(4) Celestial Caller, Unit, \#16**  
**3/3**  
**Divine Shield**  
**Taunt**  
**Armor 3**  
**Cry:** Summon a **Chosen by the Sun** to the left of this and a **Chosen by the Moon** to the right of this  
\~\~\~  
**6/6**  
**Divine Shield**  
**Taunt**  
**Armor 6**  
**Cry:** Summon a **Radiant** **Chosen by the Sun** to the left of this and a **Radiant** **Chosen by the Moon** to the right of this

**(2) Flaming Felinor, Unit, Felinor, \#17**  
3/6  
**Cry:** deal 3 damage to all other units  
\~\~\~  
6/12  
**Cry:** deal 6 damage to all enemy units

**(1) Knife Juggler, Unit, Human, \#18**  
3/2  
**Pierce**  
When you summon a unit deal 1 damage to a random enemy  
*{spawn voice line: put this apple on your head}*  
\~\~\~  
6/4  
**Pierce**  
When you summon a unit deal 3 damage to a random enemy

**(1) Extra Rations, Field Spell, \#19**  
**Start of turn:** Draw 1  
\~\~\~  
**Start of turn:** Draw 2, they cost (1) less

**(2) Recruitment Posters, Field Spell, \#20**  
**Start of turn:** Add a random **Training-grounds** card to your hand  
\~\~\~  
**Start of turn:** Add a random **Radiant Training-grounds** card to your hand

**(1) Flame Tongue Totem, Unit, \#21**  
0/4  
Adjacent units have \+3 attack  
\~\~\~  
0/8  
Adjacent units have \+6 attack

**(2) Right-Hand Felinor, Unit, Felinor, \#22**  
3/3  
**Cry:** if placed to the right of a unit give this and that unit \+3/+3  
\~\~\~  
6/6  
**Cry:** if placed to the right of a unit give this and that unit \+6/+6

**(3) Quadrupling Felinors, Unit, Felinor, \#23**  
3/4  
**Cry:** Summon 3 copies of this unit  
\~\~\~  
6/8  
**Cry:** Summon 3 copies of this unit

**(1) Goliath Situation, Field Trap, \#24**  
**Reveals** when your opponent plays a unit, remember its cost.  
**On reveal:** cards in your hand that cost less than the remembered cost, cost (0) instead.  
\~\~\~  
**Reveals** when your opponent plays a unit that costs (2) or more, remember its cost.  
**On reveal:** cards in your hand that cost less than or equal to the remembered cost, cost (0) instead.

**(3) Trigor, Unit, Human, \#25**  
**Tribute 3**  
3/3  
Death: summon a **Trigor** with \+3/+3  
\~\~\~  
**Tribute 3**  
6/6  
Death: summon a **Trigor** with \+6/+6

**(4) Path of Exile, Spell, \#26**  
**Exile** a permanent, draw a card  
\~\~\~  
**Exile** a permanent, draw 2 cards  
**Cleave**

**(4) Hopeful Jelly Bean, Spell, Jelly Bean, \#27**  
Make a permanent **Radiant**  
\~\~\~  
Make a permanent **Radiant**  
**Cleave**

**(3) Occidental, Human, \#28**  
9/5  
**Cry:** Destroy all permanents in the right most lane  
\~\~\~  
18/10  
**Cry:** Destroy all enemy permanents in the right most lane

**(3) Oriental, CN, \#29**  
5/9  
**Cry:** Destroy all permanents in the left most lane  
\~\~\~  
10/18  
**Cry:** Destroy all enemy permanents in the left most lan

**(3) Floodgates, Unit, \#30**  
0/12  
**Armor 3**  
Whenever a unit is summoned gain **\+1 Armor**  
Death: destroy all friendly units  
\~\~\~  
0/24  
**Armor 6**  
Whenever a unit is summoned gain **\+2 Armor**  
Death: destroy all friendly units

**(3) Level Up\!, Field Spell, \#31**  
After your units attack 3 times, they become **Radiant**   
*{start tracking after this is played}*  
\~\~\~  
After your units attack 2 times, they become **Radiant**  
*{start tracking after this is played}*

**(1) Selfless Protector, Unit, Human, \#32**  
2/1  
**Death:** give your units **Divine Shield**  
\~\~\~  
4/2  
**Cry & Death:** give your units **Divine Shield**

**(3) Guerilla Felinor, Unit, Felinor, \#33**  
4/6  
**Cry:** Deal 1 damage to a random enemy unit, repeat 7 times.  
\~\~\~  
8/12  
**Pierce**  
**Cry:** Deal 1 damage to a random enemy unit, repeat 14 times.

**(3) Dino-size, Spell, \#34**  
Set a units stats to 10/10  
\~\~\~  
Set a units stats to 20/20

**(3) Corpse Recycler, Field Spell, \#35**  
Non token units on death summon a **Rush token**  
\~\~\~  
Non token units on death summon a **Radiant Rush token**

**(1) Training Dummy, Unit, \#36**  
0/3  
**Can’t be in defense position**  
**Death:** Add a random **Radiant Tutorial-Grounds** card to your hand  
\~\~\~  
0/6  
**Death:** Add two random **Radiant Tutorial-Grounds** card to your hand

**(2) Seance, Spell, \#37**  
Add an exact copy of a permanent to your hand  
*{preserves cost changes}*  
\~\~\~  
Add an exact copy of a permanent to your hand, it costs (0)

**(4) Tutorial Master, Unit, Human, \#38**  
4/16  
**Cry & Death:** Add 3 random **Training-Grounds** cards to your hand, they cost (1) less  
\~\~\~  
8/32  
**Cry & Death:** Add 3 random **Radiant** **Training-Grounds** cards to your hand, they cost (1) less 

**(4) Call to Chaos (Training-Grounds Edition) Spell, Call to Chaos, Legendary, \#39**  
???  
*(One of the following random effects)*

- Summon 3 Rush tokens, all units in your hand, field, and deck gain **Taunt** and **Divine Shield**  
- Fill your hand with **Goetzmann Game Design** and refresh 4 mana  
- Add 4 **Training-Grounds** edition cards to your hand, they cost (0)  
- Gain 8 mana and draw 2 cards  
- Summon 6 random **Training-Grounds** permanents   
- Add 2 **Radiant** **Training-Grounds** edition cards to your hand, they cost (0)  
- Summon 5 Rush tokens, for the rest of the game your units have **Rush, Trample,** and **Pierce**  
- Summon a **Training-Grounds Golem**  
- Now and for the rest of the game, every other turn add a **Call to Chaos** to your hand, it costs (0)  
- Cast a random **Call to Chaos**

\~\~\~  
\!\!\!  
*(three effects)*

**(4) Training-Grounds Golem Unit, Token, Legendary, \#39.1**  
10/10  
**Rush, Armor 10, Taunt, Divine Shield, Pierce**  
\~\~\~  
20/20  
**Rush, Armor 20, Taunt, Divine Shield, Pierce**

**(0) Goetzmann Game Design, Spell, Quickdraw, \#40**  
Add a random card to your hand  
\~\~\~  
Add a random **Radiant** card to your hand

**These 20 cards: (SNAP SHOT THEM SO THEY ARE LIKE LABELED \#41,\#42 etc** \[they should be separate entities\])  
1 Bigot  
2 Stockpile  
3 Mr vanilla  
4 Duplicating felinors  
5 Jlockheed weapons  
6 Hit job  
7 Midranges menace  
8 Jlockheed shredder  
9 Reoccuring dream  
10 Glowy Jelly Bean  
11 Big felinor  
12 Snom bunny mind control   
13 prejudice post doc  
14 the rock  
15 reminisce   
16 corpse eater  
17 recycling initiative  
18 true strike  
19 fig of life  
20 Rush token farm  
```
