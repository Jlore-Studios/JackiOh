# Stinkyscape+

## Stinkyscape+ Supplementary Balance Changes

* **Jay Fungus**  
  * Gains the tag “**Swarm”**   
  * Normal gains: **Evolve**: **Poisonous & Evolves** when your hand has at least (3) cards that costs (0)  
  * Normal gains: **Evolve**: **Poisonous & Evolves** when your hand has at least (1) card that costs (0)  
* **Selective Breeding**  
  * Gains: **Evolve** all cards in your deck  
* **Rotten Grape**  
  * Gains: **Tipsy 1**  
* **Money Machine**  
  * New text  
    * **Cry**: Print either **Coins** or **Warhero Tokens**   
      * {For AI: defaults to The Coin if the Cry doesn’t go off}   
    * **End of Turn**: Shuffle the printed \<coin choice here\> into both decks. Flip yours. If heads, put it in your hand instead.  
  * Radiant: Gains **Lucky 1** and prints Radiant versions  
* **xxFreshJack**  
  * BUSTED cards include cards from past patches too.  
  * Should use winrates when evaluating what’s busted.   
  * Create a general function for figuring out what’s BUSTED so this doesn’t have to be maintained every update.  
* **Heroic Power**  
  * Reduce costs of existing powers by (1) across the board  
  * New Powers  
    * **(10) End Times**  
      * Normal: Discover a Terminus card. End of Turn: Reduce the cost of the next use of this by (1).  
      * Radiant: Discover a Terminus card. End of Turn: Reduce the cost of the next use of this by (2).  
* **Great Paperclip Theory**  
  * Gains the tag **Paperclip**  
  * New name: **Great Paperclip Theory I**

## Stinkyscape+ Mechanics 

### Keywords

* **Evolve/Evolves**  
  * **Evolve:** An additional effect/stat change that happens when it is **Evolved**  
  * **Evolves:** A condition that causes a unit to **Evolve**  
  * You can only **Evolve** once  
* **Refresh**  
  * Gives you back your non-**Refresh** giving activates again  
  * E.g., if you **Refresh** an **Activate 4** you would get another **4 Activates**  
  * Word in the way that makes the most sense  
  * Existing “Refresh mana” instances now become “Replenish mana” to avoid confusion  
* **Tipsy X**  
  * A for fun drawback condition that takes the form of a player counter like Jade or Credits  
    * Should be in a stein shape  
  * While active, new cards played, summoned, or ones you draw may occasionally have a number or keyword or word in their name blurred on the client side  
  * Chance scales with the X  
  * Decays by 1 at the start of your turn  
  * When it reaches 0, everything clears. Doesn’t clear from going from 2-\>1 though, just the odds of new things blurring goes down  
  * Counter increases by X when playing a card with Tipsy X  
* **GIGA**  
  * **GIGA** applies to permanents. They make them 2x2 (visually and actually) taking up two backrow and two unit lanes. It can be applied to traps field spells and units. It is technically part of both lanes basically making it twice as susceptible to cleave and can be lane eated in anything else.  
  * You have a lot of freedom to design this but the main thing is that they are supposed to be BIG.   
* **Warhero Token (Stinkyscape Resource)**  
  * At the beginning of the game, all Warhero tokens are worth 0 credits.  
  * As soon as a single Warhero Token is printed, at the end of every turn, all Warhero tokens are worth 0-10 more credits.  
    * Radiant versions are worth double whatever this counter is  
  * They do nothing when played other than give credits.  
  * Cost: (0)  
  * Rarity: Rare  
  * Flavor text: “Yeah, I’m a war hero.”  
* **Flexible (New Keyword)**  
  * Can be played in the unit row or the back row  
* **Frenzy (New Keyword)**  
  * **Frenzy** \[effect\]  
  * **Frenzy** triggers the effect after the unit attacks and destroys a unit  
  * So for example **Frenzy** can attack again would equate to **After this kills a unit it may attack again**  
  * Give frenzy to all past cards with this effect  
  * Nerfs can get rid of **Frenzy** keyword  
  * Unit being buffed or gaining a random key word can gain the combat keyword **Frenzy** with “**Frenzy** this may attack again” and this  
* **Evasive (New Keyword)**  
  * Same thing as **Can’t be attacked**  
  * Give all units with **Can’t be attacked** this  
  * Units with **Evasive** implicitly have **Can’t Defend** (similar to indestructible)  
  * A nerf can remove **Evasive**   
* **Burns (New Keyword)**  
  * **Burns \[int\]**   
  * **Burn**ed units/heroes take \[int\] *unpierced* damage at the start of their owners turn, then the burn is decremented by 3 *{eg burn 8 \-\> burn 5 \-\> burn 2 \-\> no burn}*  
  * Can also get *Damage dealt Burns* which converts all damage to **Burn**   
* **Toxic (New Keyword)**  
  * **Toxic \[int\]**   
  * **Toxic** units/heroes take \[int\] *pierce* damage at the start of their owners turn in perpetuity   
  * Can also get *Damage dealt is Toxic* which converts all damage to **Toxic** 

### Misc Changes

* **New AI Cards, (Should have tags)**  
  * Create 2 New AI cards that are also fruits  
  * Create 2 New AI cards that Jlockeed  
  * Create 2 New AI cards that are CN  
  * Create 1 New AI card that is KY

### New Cards 

**(1) Starfruit, Spell, Fruit, Common, Stinkyscape+, \#1**  
**Cleave, Pierce**  
Deal 1 damage to a Unit.  
\~\~\~   
**Cleave, Pierce, Lifesteal**  
Deal 2 damage to a Unit.

**(1) Lone Drifter, Unit, Swarm, Common, Stinkyscape+, \#2**  
2/2  
**Evolves** when all your Unit zones are full.  
**Evolve**: Gain \+4/+4.  
\~\~\~   
4/4  
**Evolves** when all your Unit zones are full.  
**Evolve**: Gain \+8/+8.

**(3) High Evolutionary, Unit, Swarm, Legendary, Stinkyscape+, \#3**  
7/7  
**Cry**: Evolve a friendly permanent.  
**Aura**: Your evolutions trigger twice.  
\~\~\~  
14/14  
**Cry**: Evolve a friendly permanent.  
**Aura**: Your evolutions trigger thrice.

**(4) Profe Yates, Unit, Catalyst, Acclaimed, Human, Felinor, Legendary, Stinkyscape+, \#4**  
7/7  
**Cry**: Exile every card with a M, O, U, S, E in its name *{cards in CN are immune}* then summon two random **traps** then choose one of the following **Activates:**

- Activate 1: Overwhelm: Freeze all units in this lane & surrounding lanes  
- Activate 3: Wolf’s Maw: Pull a flying unit down *{it loses flying}*, and deal 4 damage to it  
- Activate 1: Iron Mandate: Fuse this into a unit, *{does not trigger this cry again}*

*{Spawn voiceline: Every pack needs an alpha I don't see anyone else stepping up}*  
*{Attack voiceline: a mouse once bit my sister}*  
*{Death voiceline: I will not see the Northern Lights again}*  
\~\~\~  
14/14  
**Cry**: Exile every card with a M, O, U, S, E in its name *{cards in CN are immune}* then summon three random **Radiant** **traps** hen choose one of the following **Activates:**

- Activate 1: Overwhelm: Freeze all units in this lane & surrounding lanes  
- Activate 3: Wolf’s Maw: Pull a flying unit down *{it loses flying}*, and deal 4 damage to it  
- Activate 1: Iron Mandate: Fuse this into a unit, *{does not trigger this cry again}*

**(2) Dingleberry Droat, Unit, Acclaimed, Human, Legendary, Stinkyscape+, \#5**  
2/1  
**Cry: Fuse** Fruits together until you make one that costs at least (5). Add it to your hand. It costs (2).  
\~\~\~  
4/2  
**Cry: Fuse** Radiant Fruits together until you make one that costs at least (5). Add it to your hand. It costs (2).

**(2) Wines McCheese, Unit, Human, Epic, Stinkyscape+, \#6**  
4/5  
**Start of Turn:** Add a Fine Fermented Grape, Stinky Cheese, or The Coin to your hand at random.  
\~\~\~  
8/10  
**Start of Turn:** Add a Radiant Fine Fermented Grapes, Stinky Cheese, or The Coin to your hand at random.

**(1) Fine Fermented Grape, Spell, Fruit, Token, Epic, Stinkyscape+, \#6.1**  
**Tipsy 2**  
**End of Turn**: Fuse a different Grape into this.  
\~\~\~  
**Tipsy 2**  
**End of Turn**: Fuse a different Radiant Grape into this.

**(1) Stinky Cheese, Field Spell, Swarm, Token, Epic, Stinkyscape+, \#6.2**  
**Temporary 2**  
**Evolves** when this is discarded to **Temporary**.  
**Evolve**: Gain **Flashback**, **Animated**, and \+5/+5  
\~\~\~  
**Temporary 1**  
**Evolves** when this is discarded to **Temporary**.  
**Evolve**: Gain **Flashback**, **Animated**, and \+10/+10

**(4) Churnwalker, Unit, Legendary, Stinkyscape+, \#7**  
3/15   
**Armor 4**  
**Cry:** Choose one

- Chain all enemy units together, when one takes damage all other chained units also take the damage  
- Chain all friendly units together, when one takes damage it is split between all allies (rounded down)

\~\~\~  
6/30   
**Armor 8**  
**Cry:** 

- Chain all enemy units together, when one takes damage all other chained units also take the damage  
- Chain all friendly units together, when one takes damage it is split between all allies (rounded down)

**(2) Drone Assembler v3, Unit, Rare, Weapon, Stinkyscape+, \#8**  
\[0/2\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target an enemy, summon a **Drone Interceptor** to attack it  
\~\~\~  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target an enemy, summon a **Radiant Drone Interceptor** to attack it

**(3) Drone Assembler v4, Unit, Epic, Weapon, Stinkyscape+, \#9**  
\[0/3\]  
**Starship Piece**  
**Equip**  
**Activate 2:** Target an enemy, summon a **Drone Interceptor** to attack it  
\~\~\~  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 2:** Target an enemy, summon a **Radiant Drone Interceptor** to attack it

**(4) Drone Assembler v5, Unit, Legendary, Weapon, Stinkyscape+, \#10**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target an enemy, summon a **Drone Corvette** to attack it  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 1:** Target an enemy, summon a **Radiant Drone Corvette** to attack it

**(3) Energy Booster, Unit, Epic, Stinkyscape+, \#11**  
1/1  
**Starship Piece**  
**Equip**  
**Activate 1: Refresh** *{e.g., if its activate 4, you may use this and then you’d get another 4 activates}*  
\~\~\~  
2/2  
**Starship Piece**  
**Equip**  
**Activate 2: Refresh** *{e.g., if its activate 4, you may use this and then you’d get another 4 activates}*

**(1) Shield Booster, Unit, Common, Stinkyscape+, \#12**  
1/1  
**Starship Piece**  
**Equip**  
**Divine Shield**  
\~\~\~  
2/2  
**Starship Piece**  
**Equip**  
**Divine Shield**

**(2) Shield Repairer, Unit, Rare, Stinkyscape+, \#13**  
2/2  
**Starship Piece**  
**Equip**  
**End of turn:** gain **Divine Shield**  
\~\~\~  
4/4  
**Starship Piece**  
**Equip**  
**Divine Shield**  
**End of turn:** gain **Divine Shield**

**(1) Hull Repairer, Unit, Common, Stinkyscape+, \#14**  
0/2  
**Starship Piece**  
**Equip**  
**End of turn:** Heal this 4 health  
\~\~\~  
0/4  
**Starship Piece**  
**Equip**  
**End of turn:** Heal this 8 health

**(2) Hull Repairer mk2, Unit, Rare, Stinkyscape+, \#15**  
0/4  
**Starship Piece**  
**Equip**  
**End of turn:** Heal this to full health  
\~\~\~  
0/8  
**Starship Piece**  
**Equip**  
**End of turn:** Heal this to full health, and gain \+2 max health

**(2) Servo Surge, Unit, Rare, Stinkyscape+, \#16**  
2/1  
**Starship Piece**  
**Equip**  
**Windfury**  
\~\~\~  
4/2  
**Starship Piece**  
**Equip**  
**Windfury**

**(3) Smoothie, Spell, Rare, Fruit, Stinkyscape+, \#17**  
There is X & Y inside the smoothie *{when added to your hand there select two random fruits to be added to the smoothie, their targets are merged together but essentially works like a fused card}*  
\~\~\~  
There is **Radiant** X & **Radiant** Y inside the smoothie 

**(4) Large Smoothie, Spell, Rare, Fruit, Stinkyscape+, \#18**  
There is X, Y, & Z inside the smoothie *{when added to your hand there select three random fruits to be added to the smoothie, their targets are merged together but essentially works like a fused card}*  
\~\~\~  
There is **Radiant** X, **Radiant** X, & **Radiant X** inside the smoothie 

**(2) Normalization Module, Unit, Rare, Stinkyscape+, \#19**  
0/2  
**Starship Piece**  
**Equip**  
**Activate: Vanilla** a unit  
\~\~\~  
0/4  
**Starship Piece**  
**Equip**  
**Activate 2: Vanilla** a unit

**(1) B-Patch, Spell, Epic, Stinkyscape+, \#20**  
**Cast on draw:** Randomly either **buff** or **nerf** *{50/50}*, every card on the field and in both hands  
\~\~\~  
**Cast on draw:** Randomly either **buff** or **nerf** *{50/50}*, every card on the field and in both hands, **Lucky 1**

**(4) Jlockeed Shredder-12 , Unit, Rare, Jlockeed, Stinkyscape+, \#21**  
12/16  
**Tribute 1**  
**Pierce**  
**End of turn:** Deal 2 damage to all enemies, twice  
\~\~\~  
24/32  
**Tribute 1**  
**Pierce**  
**End of turn:** Deal 5 damage to all enemies, twice

**(1) Cryofluid, Spell, Mindustrial, Rare, Mindustrial, Stinkyscape+, \#22**  
Give a Unit \+6/+6, **Deft**, and **Freeze** it.  
\~\~\~  
Give a Unit \+12/+12, **Deft**, and **Freeze** it.

**(3) GIGA-ify, Spell, Epic, Stinkyscape+, \#23**  
Make a permanent in your hand **GIGA** and **Indestructible.**  
\~\~  
Make two permanents in your hand **GIGA** and **Indestructible**

**(1 embiggen 3\) Forced Propagation, Spell, Epic, Stinkyscape+, \#24**  
Choose a non-**fused** card in your hand or field, shuffle 10 embiggen 40 copies of a card into your deck *{does no preserve cost changes or buffs on them}*  
\~\~\~  
Choose a non-**fused** card in your hand or field, shuffle 10 embiggen 40 copies of a card into your deck, then **buff** them *{does no preserve cost changes or buffs on them}*

**(3) Snom Bunny, Unit, Human, Epic, Stinkyscape+, \#25**  
5/5  
**Can’t be in defense position**  
**Death:** A random enemy **Permanent** comes under your control  
\~\~\~  
10/10  
**Lucky 1**  
**Death:** A random enemy **Permanent** comes under your control

**(1) Jlockeed’s Forever War, Spell, Jlockeed, Rare, Stinkyscape+, \#26**  
For the rest of the game, at the **start of your turn** summon a Conscript.  
\~\~\~  
For the rest of the game, at the **start of your turn** summon a **Radiant** conscript.

**(1) Conscript, Unit, Human, Token, Common, Stinkyscape+, \#26.1**  
1/1  
**Ranged**  
\~\~\~  
2/2  
**Ranged, Rush, Pierce**

**(3) Gargantuar, Unit, Swarm, Rare, Stinkyscape+, \#27**  
12/12  
After this attacks, Freeze it.  
**Evolves** when this falls below half health.  
**Evolve**: Summon an Imp.  
\~\~\~  
24/24  
**Evolves** when this falls below half health.  
**Evolve**: Summon a Radiant Imp.

**(1) Imp, Unit, Swarm, Token, Common, Stinkyscape+, \#27.1**  
1/1  
**Can’t Defend**  
**Charge**  
\~\~\~  
2/2  
**Can’t Defend**  
**Charge**  
**Pierce**

**(0) Zombie-Box, Spell, Rare, Common, Stinkyscape+, \#28**  
Nerf your hand. Summon a Dormant 1 **Gargantuar**.  
\~\~\~  
Nerf your hand. Summon a Radiant **Gargantuar**.

**(2) Grinder Veteran, Unit, Common, Stinkyscape+, \#29**  
5/5  
**Deft**  
**Activate:** Discard a Felinor to add 2 **Warhero Tokens** to your hand.  
\~\~\~  
10/10  
**Deft**  
**Activate:** Discard a Felinor to add 2 Radiant **Warhero Tokens** to your hand.

**(1) Yeah I’m a War Hero, Field Spell, Rare, Stinkyscape+, \#30**  
The first card you draw each turn is replaced with a **Warhero Token**.  
\~\~\~  
The first card you draw each turn is replaced with a Radiant **Warhero Token**.

**(1) Soul Read, Trap, Common, Stinkyscape+, \#31**  
**Cry:** Look at your opponent's hand and choose a card secretly.  
Reveals if your opponent plays the chosen card before any of the other cards you saw. Take control of that card instead. Otherwise, Tribute this Trap.  
\~\~\~  
**Cry:** Look at your opponent's hand and choose a card secretly.  
Reveals if your opponent plays the chosen card. Take control of that card instead.

**(3) Z-Patch, Spell, Epic, Stinkyscape+, \#32**  
**Tradable**  
Buff every permanent 5 times. Then Nerf every permanent 5 times.  
\~\~\~  
**Tradable**  
Buff every permanent 5 times. Then Nerf every enemy permanent 5 times.  
{Flavor text: “Watchu know about the Z patch?”}

**(4) True Conservative, Unit, Human, Legendary, Stinkyscape+, \#33**  
3/3  
**Cry:** Transform ALL Units into a version from their Past.  
{For AI: look in the patch notes. If a card doesn’t have a past version, it’s untouched}  
\~\~\~  
6/6  
**Cry:** Transform ALL Units into a version from their Past. Yours become their strongest version.

**(2) Neoconservative, Unit, Human, Common, Stinkyscape+, \#34**  
5/3  
**Cry**: Add a Jlockeed card from the past to your hand.  
\~\~\~  
10/6  
**Cry**: Add a Radiant Jlockeed card from the past to your hand.

**(2) Scatter, Unit, Mindustrial, Common, Stinkyscape+, \#35**  
**Flexible**  
**Can’t attack**  
**Can’t defend**  
0/6  
**Activate 1:** Deal 0\~2 damage to all enemies, if the unit is flying deal 8 damage instead  
\~\~\~  
**Flexible**  
**Can’t attack**  
**Can’t defend**  
0/12  
**Activate 2:** Deal 1\~3 damage to all enemies, if the unit is flying deal 10 damage instead

**(1) Duo, Unit, Mindustrial, Common, Stinkyscape+, \#36**  
**Flexible**  
**Can’t defend**  
2/1  
**Ranged, Windfury**  
\~\~\~  
**Flexible**  
**Can’t defend**  
4/2  
**Ranged, Windfury, Pierce**

**(1) Scorch, Unit, Mindustrial, Common, Stinkyscape+, \#37**  
**Can’t defend**  
Can attack 10 times per turn  
Can only attack the unit directly across from this  
**Ranged**  
**Pierce**  
1/1  
\~\~\~  
**Can’t defend**  
Can attack 10 times per turn  
Can only attack the unit directly across from this  
**Ranged**  
**Pierce, Cloaked**  
2/2

**(2) Arc, Unit, Mindustrial, Common, Stinkyscape+, \#38**  
**Can’t defend**  
**Activate 2:** Deal 2 damage to a unit, this randomly chains to an adjacent unit *{e.g., can change to a unit if in an adjacent lane or in the backrow, repeats until no valid targets, cannot chain to the same unit twice in one activate}*  
0/5  
\~\~\~  
**Can’t defend**  
**Activate 5:** Deal 2 damage to a unit, this randomly chains to an adjacent unit *{e.g., can change to a unit if in an adjacent lane or in the backrow, repeats until no valid targets, cannot chain to the same unit twice in one activate}*  
0/10

**(1) Mindustrial Cross-over, Spell, Rare, Stinkyscape+, \#39**  
**Discover** a random **Mindustrial Unit,** give it **Starpiece** & **Equip**  
\~\~\~  
**Discover** a random **Radiant** **Mindustrial Unit,** give it **Starpiece** & **Equip,** it costs (1) less

**(1) Mender, Field Spell, Mindustrial, Common, Stinkyscape+, \#40**  
**Flexible**  
**End of turn:** Heal adjacent units to full  
\~\~\~  
**Flexible**  
**End of turn:** Heal adjacent units to full & give them **Divine Shield**

**(2) Mend Projector, Field Spell, Mindustrial, Rare, Stinkyscape+, \#41**  
**End of turn:** Heal all units to full  
\~\~\~  
**End of turn:** Heal all units to full & give them **Divine Shield**

**(3) Force Projector, Field Spell, Mindustrial, Rare, Stinkyscape+, \#42**  
The first 5 damage allies would take per turn is nullified *{5 left}*  
\~\~\~  
**Armor 2**  
The first 10 damage allies would take per turn is nullified *{accounts for armor}* *{10 left}*

**(2) Segment, Field Trap, Mindustrial, Rare, Stinkyscape+, \#43**  
The first enemy **Ranged attack** or **Activate** per turn is countered *{they deal no damage, doesn’t go through}*  
\~\~\~  
All enemy **Ranged attack** or **Activate** are countered *{they deal no damage, doesn’t go through}*

**(1) Blast Compound, Spell, Mindustrial, Mineral, Rare, Stinkyscape+, \#44**  
Give a unit **\+2** attack and **Cleave**, **lock** the tile it is on  
\~\~\~  
Give a unit **\+6** attack and **Cleave**, **lock** the tile it is on

**(1) Surge Alloy Spell, Mindustrial, Mineral, Legendary, Stinkyscape+, \#45**  
Give a unit **Rush** attack and **Windfury**  
\~\~\~  
Give a unit **Charge** attack and **Windfury**

**(1) Phase Fabric Spell, Mindustrial, Mineral, Epic, Stinkyscape+, \#46**  
Give a unit **Immutable** and **Immune to Spells**   
\~\~\~  
Give a unit **Immutable, Immune to Spells,** and **Can’t be Attacked**

**(1) Plastanium Spell, Mindustrial, Mineral, Epic, Stinkyscape+, \#47**  
Give a unit **\+2** health and **Divine Shield**  
\~\~\~  
Give a unit **\+6** health and **Divine Shield**

**(1) Thorium Spell, Mindustrial, Mineral, Epic, Stinkyscape+, \#48**  
Give a unit **\+5/+5** and **Brittle 1**  
\~\~\~  
Give a unit **\+10/+10** and **Brittle 1**

**(1) Metaglass, Spell, Mindustrial, Mineral, Rare, Stinkyscape+, \#49**  
Give a unit **Ranged**  
\~\~\~  
Give a unit **Ranged** and **\+3** attack

**(1) Graphite, Spell, Mindustrial, Mineral, Rare, Stinkyscape+, \#50**  
Give a unit **Piece, Deft, Frenzy** this may attack again  
\~\~\~  
Give a unit **Piece, Deft, Frenzy** this may attack again, & **First Strike**

**(2) Chains Bind Me, Spell, Common, Stinkyscape+, \#51**  
**Dormant** a permanent for 2 turns. When it awakens, Buff it 2 times if it’s yours or Nerf it 2 times if it’s an enemy’s.  
\~\~\~  
**Dormant** a permanent for 2 turns. When it awakens, Buff it 5 times if it’s yours or Nerf it 5 times if it’s an enemy’s.

**(4) Airblast Drill, Field Spell, Mindustrial, Legendary, Stinkyscape+, \#52**  
**GIGA**  
**Tribute 1**  
**End of turn:** Fill your hand with random **Minerals** (that cost 0\) and **Resources**  
\~\~\~  
**GIGA**  
**Tribute 1**  
**End of turn:** Fill your hand with random **Radiant** **Minerals** (that cost 0\) and **Radiant** **Resources**

**(4) Laser Drill, Field Spell, Mindustrial, Epic, Stinkyscape+, \#53**  
**GIGA**  
**End of turn:** Add three random **Minerals** (that cost 0\) or **Resources** to your hand  
\~\~\~  
**GIGA**  
**GIGA**  
**End of turn:** Add three random **Radiant** **Minerals** (that cost 0\) or **Radiant** **Resources** to your hand

**(2) Pneumatic Drill, Field Spell, Mindustrial, Rare, Stinkyscape+, \#54**  
**End of turn:** Add a random **Minerals** or **Resources** to your hand  
\~\~\~  
**End of turn:** Add a random **Radiant** **Minerals** or **Radiant** **Resources** to your hand

**(2) Thorium Reactor, Field Spell, Mindustrial, Epic, Stinkyscape+, \#55**  
**Start of turn:** Gain 3 mana & draw 1 card  
**End of turn:** 15% chance to blow up dealing 30 damage to your hero  
**Activate:** Discard a **Thorium** to skip the blow up chance this turn  
\~\~\~  
**Lucky 1**  
**Start of turn:** Gain 3 mana & draw 1 card  
**End of turn:** 15% chance to blow up dealing 30 damage to your hero   
**Activate:** Discard a **Thorium** to skip the blow up chance this turn

**(3) Impact Reactor, Field Spell, Mindustrial, Legendary, Stinkyscape+, \#56**  
**GIGA**  
**Cry:** lose 4 mana next turn, and 2 mana the turn after  
**Dormant for 2 turns**  
**When awoken:** Gain infinite mana  
\~\~\~  
**GIGA**  
**Cry:** lose 2 mana next turn, and 1 mana the turn after  
**Dormant for 2 turns**  
**When awoken:** Gain infinite mana

**(1) Phase Weaver, Field Spell, Mindustrial, Epic, Stinkyscape+, \#57**  
**End of your turn:** add a **Phase Fabric** to your hand  
\~\~\~  
**End of your turn:** add a **Phase Fabric** to your hand, it costs (0)

**(1) Phase Conveyer, Unit, Mindustrial, Epic, Stinkyscape+, \#58**  
**Flexible**  
0/3  
**Start of turn:** add a **Mineral** and **Resource** to your hand  
\~\~\~  
**Flexible**  
0/6  
**Start of turn:** add a **Radiant** **Mineral** and a **Radiant** **Resource** to your hand

**(2) Mass Driver, Field Spell, Mindustrial, Epic, Stinkyscape+, \#59**  
**Flexible**  
0/6  
**Activate:** in 2 turns add two **Minerals** that cost (0) and a **Resource** to your hand  
\~\~\~  
**Flexible**  
0/12  
**Activate:** in 2 turns add two **Radiant Minerals** that cost (0) and a **Radiant Resource** to your hand

**(2) Radi, Unit, Rare, Stinkyscape+, \#60**  
\[1/1\]  
**Cry:** Discover 1 **Rare** **Weapons**; **Equip** them into this  
\~\~\~  
\[2/2\]  
**Cry:** Discover 1 **Epic** **Weapons**; **Equip** them into this

**(2) Jayaad, Unit, Acclaimed, Epic, Stinkyscape+, \#61**  
2/6  
**Cry:** Enact **Shawarma Law** *{summon **Shawarma Law**}*  
\~\~\~  
4/12  
**Cry:** Enact Enhance **Shawarma Law** *{summon Radiant **Shawarma Law**}*

**(2) Shawarma Law, Field Spell, Epic, Stinkyscape+, \#61.1**  
**End of turn:** Every unit eats a delicious **Shawarma** causing one of the following effects:

- First **Shawarma** eaten gain **\+3 health** *{Shawarma was very tasty}*  
- Second **Shawarma** eaten gain **Can’t attack** *{too busy eating Shawarma}*  
- Third **Shawarma** eaten gain **Can’t defend** and **Brittle 2** *{Ate too much Shawarma}*  
- Subsequent **Shawarma** eaten **Freeze** *{Shawarma overload}*

\~\~\~  
**End of turn:** Every unit eats a delicious **Shawarma** causing one of the following effects:

- First **Shawarma** eaten gain **\+3 health** *{Shawarma was very tasty}*  
- Second **Shawarma** eaten gain **Can’t attack** *{too busy eating Shawarma}*  
- Third **Shawarma** eaten gain **Can’t defend** and **Brittle 2** *{Ate too much Shawarma}*  
- Subsequent **Shawarma** eaten **Freeze** *{Shawarma overload}*

Ally units gain discipline and stop at one **Shawarma** 

**(1) Overdrive Projector, Field Spell, Mindustrial, Epic, Stinkyscape+, \#62**  
**Dormant for 1 turn**  
All units gain **Rush** and **Windfury** and Spells gains **Echo \+1**  
\~\~\~  
**Dormant for 1 turn**  
Your units gain **Rush** and **Windfury** and Spells gains **Echo \+1**

**(2) Overdrive Dome, Field Spell, Mindustrial, Epic, Stinkyscape+, \#63**  
**Dormant for 2 turns**  
All units gain **Charge** and **Windfury** and Spell gains **Echo \+2**  
\~\~\~  
**Dormant for 2 turns**  
Your units gain **Charge** and **Windfury** and Spell gains **Echo \+2**

**(1 embiggen 4\) Surge Wall, Unit, Mindustrial, Legendary, Stinkyscape+, \#64**  
0/6 embiggen 0/24  
**Can’t attack**  
**Can’t defend**  
**End of turn** deal 1 embiggen 2 damage to all enemies  
**Embiggen: GIGA**  
\~\~\~  
0/12 embiggen 0/48  
**Can’t attack**  
**Can’t defend**  
**End of turn** deal 2 embiggen 4 damage to all enemies  
**Pierce, Armor 2**  
**Embiggen: GIGA**

**(1 embiggen 4\) Thorium Wall Unit, Mindustrial, Epic, Stinkyscape+, \#65**  
0/8 embiggen 0/32  
**Can’t attack**  
**Can’t defend**  
**Embiggen: GIGA**  
\~\~\~  
0/16 embiggen 0/64  
**Can’t attack**  
**Can’t defend**  
**Armor 2**  
**Embiggen: GIGA**

**(1 embiggen 4\) Plastanium Wall Unit, Mindustrial, Epic, Stinkyscape+, \#66**  
0/4 embiggen 0/16  
**Can’t attack**  
**Can’t defend**  
**End of turn:** Gain **Divine Shield**  
**Embiggen: GIGA**  
\~\~\~  
0/8 embiggen 0/32  
**Can’t attack**  
**Can’t defend**  
**End of turn:** Gain **Divine Shield**  
**Armor 2**  
**Embiggen: GIGA**

**(1 embiggen 4\) Phase Wall Unit, Mindustrial, Epic, Stinkyscape+, \#67**  
0/6 embiggen 0/24  
**Can’t attack**  
**Can’t defend**  
**Immutable, Immune to Spells**  
**Embiggen: GIGA**  
\~\~\~  
0/12 embiggen 0/48  
**Can’t attack**  
**Can’t defend**  
**Immutable, Immune to Spells**  
**Armor 2**  
**Embiggen: GIGA**

**(1) Shadiris Visions, Spell, Rare, Stinkyscape+, \#68**  
**Discover** a **Spell** in your deck; add a copy of it to your hand  
\~\~\~  
**Discover** a **Spell** in your deck; add a **Radiant** copy of it to your hand, it costs (1) less

**(2) Exotic Jellybean, Spell, Epic, Jellybean, Stinkyscape+, \#69**  
Add a **Sweet Essence** to your hand  
\~\~\~  
Add a **Radiant** **Sweet Essence** to your hand

**(0) Sweet Essense, Spell, Epic, Jellybean, Stinkyscape+, \#69.1**  
If this is the left most card in your hand, the card to the right of this is **Radiant**  
\~\~\~  
If this is the left most card in your hand, the card to the right of this is **Radiant** and costs (1) less  
The card to the left of this costs (1) less

**(1) Spaceberry, Spell, Epic, Fruit, Stinkyscape, \#70**  
**Cast on draw:** Shuffle 3 **Cast on draw** into your deck  
\~\~\~  
**Cast on draw:** Shuffle 3 **Radiant Cast on draw** into your deck

**(1) Mindustrial Contract, Spell, Rare, Stinkyscape+, \#71**  
Add 3 random **Mindustrial** cards to your hand  
\~\~\~  
Add 3 random **Radiant Mindustrial** cards to your hand

**(4) Foreshadow, Unit, Mythic, Mindustrial, Stinkyscape+, \#72**  
**Indestructable**  
**Can’t attack**  
**Can’t defend**  
**Immutable, Immune to Spells**  
**Flexible**  
**Tribute 5**  
**Ranged**  
20/20  
When an enemy unit is summoned attack it and lose 1 mana next turn  
\~\~\~  
**Indestructable**  
**Can’t defend**  
**Immutable, Immune to Spells**  
**Flexible**  
**Tribute 5**  
**Ranged**  
40/40  
When an enemy unit is summoned attack it and lose 1 mana next turn

**(3) Ripple, Unit, Epic, Mindustrial, Stinkyscape+, \#73**  
**Can’t attack**  
**Can’t defend**  
**Flexible**  
**Ranged**  
4/16  
When an enemy unit is summoned attack it  
\~\~\~  
**Can’t defend**  
**Flexible**  
**Ranged**  
8/32  
When an enemy unit is summoned attack it

**(2) Galaxia, Unit, Epic, Stinkyscape+, \#74**  
4/6  
**Aura**: The first Unit you play each turn gets \+3 Attack. {For AI: Including the first one played after this on the turn this is summoned.}  
\~\~\~  
8/12  
**Aura**: The first Unit you play each turn gets \+6 Attack.

**(1) Space Invader, Unit, Swarm, Common,** **Stinkyscape+, \#74**  
2/2  
**Start of Turn**: Evolve  
**Evolve**: Summon a copy of this.  
\~\~\~  
4/4  
**Starship Piece**  
**Start of Turn**: Evolve  
**Evolve**: Summon a copy of this.

**(3) J-lease S-Space EX-plorer, Unit, Human, Swarm, Legendary, Stinkyscape+, \#75**  
5/5  
**Cloaked**  
**Evolves** when you launch a Starship.  
**Evolve:** Put a **J-lease S-Space EX-plorer Pack** on top of your deck.  
\~\~\~  
10/10  
**Cloaked**  
**Evolves** when you launch a Starship.  
**Evolve:** Put a Radiant **J-lease S-Space EX-plorer Pack** on top of your deck.

**(2) J-lease S-Space EX-plorer Pack, Token, Spell, Legendary, Stinkyscape+, \#75.1**  
**Cast on Draw:** Add 5 random **Radiant** **Stinkyscape** or **Stinkyscape+** cards to your hand. *{or Stinkyscape++ cards if we add it}*  
\~\~\~  
**Cast on Draw:** Add 5 random **Radiant** **Stinkyscape** or **Stinkyscape+** cards to your hand. They cost (0)

**(2) Ayys LMAO, Spell, Common, Stinkyscape+, \#76**  
Summon a mix of 3 **Cloaked** **Ayy** tokens or **Cloaked** **Small Asteroids**. {For AI: The Ayys spawn in the backrow if possible.}  
\~\~\~  
Summon a mix of 3 Radiant **Cloaked Ayy** tokens or **Cloaked** **Small Asteroids**.

**(1) Ayy, Unit, Token, Common, Stinkyscape+, \#76.1**  
2/2  
**Cloaked**  
\~\~\~  
4/4  
**Cloaked**  
**Poisonous**

**(4) Black Hole, Spell, Terminus, Mythic, Stinkyscape+, \#77**  
Open a Black Hole in midlane. At the start of each player’s turn, it draws their permanents one lane closer. It closes after swallowing 10 permanents.  
{For AI: the Black Hole occupies all of midlane with a constant animation. Any permanent that enters midlane is exiled}  
\~\~\~  
Open a Black Hole in midlane. At the start of the opponent’s turn, it draws their permanents one lane closer.

**(4) Slashed Dream Cries in Red, Spell, Terminus, Mythic, Stinkyscape+, \#78**  
Slash the board into two *{exiling the middle lane and deleting all permanents inside, for AI replace it with a deep red mark the reveals the endless sea, units and field spells only effect their side of the board, spells have to be played on either of the two boards, only thing that is defacto the same is the hand and health}*.   
\~\~\~  
Slash the board into two, you have control over the slash *{can pick lane 2, 3, or 4}*

**(4) Tears of the Kwzki Tree, Spell, Terminus, Mythic, Stinkyscape+, \#Token**  
All cards for the rest of the game are **Radiant**, at the end of any turn buff all cards.  
\~\~\~  
All cards for the rest of the game are **Radiant**, at the end of any turn buff all cards. Gain 4 mana.

**(4) Supernova, Spell, Terminus, Mythic, Stinkyscape+, \#Token**  
Fuse all units in the board into a **Alnilam**   
\~\~\~  
Fuse all units in the board into a **Radiant** **Alnilam** 

**(4) Alnilam, Spell, Mythic, Terminus, Stinkyscape+, \#Token**  
10/10  
**Indestructable, Untributable, Immune to Spells, Brittle 3**  
**Death:** Lose the game  
\~\~\~  
20/20  
**Indestructable, Untributable, Immune to Spells, Brittle 5**  
**Death:** Lose the game

**(4) A Glimpse into the Endless Sea, Spell, Terminus, Wincon, Mythic, Stinkyscape+, \#79**  
Exile your deck. Start a 10 turn count down; upon it reaching 0 win the game.  
*{need to update bob’s plan with a custom card for this}*  
\~\~\~  
Exile your deck. Start a 8 turn count down; upon it reaching 0 win the game.

**(4) The Last Voyage, Spell, Terminus, Mythic, Stinkyscape+, \#80**  
Exile your hand, deck, and board *{including dormant cards}*; summon 10 random **Terminus Starships** *{each one should have 5\~12 mana worth of attachments, special names, different art than normal Starships, and cannot manually be launched}*. Now and the start of your turn launch a random one.   
\~\~\~  
Exile your hand, deck, and board *{including dormant cards}*; summon 10 random **Terminus Starships** *{each one should have 5\~12 mana worth of attachments, special names, different art than normal Starships, and cannot manually be launched}*. Now and the start of your turn launch a random one.   
Also exile your opponents board.

**(2) Doomsayer, Unit, Mythic, Stinkyscape+, \#81**  
0/7  
**Start of turn:** Exile your entire board and add a random **Terminus** card to your hand  
\~\~\~  
0/14  
**Start of turn:** Exile your entire board and add a random **Radiant** **Terminus** card to your hand

**(1) Endless Expedition, Spell, Mythic, Quickdraw, Stinkyscape+, \#82**  
For the rest of the game lose 1 mana. Add a random **Terminus** card to your hand, it costs (3).  
\~\~\~  
For the rest of the game lose 1 mana. Add a random **Radiant** **Terminus** card to your hand, it costs (3).

**(2) Offworld Replicant, Unit, Swarm, Common, Stinkyscape+, \#83**  
6/6  
**Stack**  
**Evolves** when this is **Stacked** on something.  
**Evolve**: Add a copy of this to your hand.  
\~\~\~  
12/12  
**Stack**  
**Evolves** when this is **Stacked** on something.  
**Evolve**: Add a copy of this to your hand.

**(1) Edwin Van’Spacechief, Unit, Human, Rare, Stinkyscape+, \#83**  
2/2  
**Combo X**: Gain \+2X/2X  
\~\~\~  
4/4  
**Combo X**: Gain \+4X/4X

**(3) Bogos Binted, Unit, Legendary,** **Stinkyscape+, \#84**  
5/6  
**Cry:** Take a snapshot of all your other permanents.  
**Death**: Reset your board to that state.  
\~\~\~  
10/12  
**Cry:** Take a snapshot of all your other permanents.  
**Death**: Reset your board to that state but they’re Radiant.

**(2) Body Snatch, Spell, Rare, Stinkyscape+, \#85**  
Transform a Unit into an **Ayy** in defense position.  
\~\~\~  
Transform a Unit into an **Ayy**.

**(0) Astrophage, Unit, Swarm, Common, Stinkyscape+, \#86**  
0/1  
**Evolve** when your opponent plays a Unit.  
**Evolve**: Place a Plague Counter on and steal a stat point from each Unit.  
\~\~\~  
0/2  
**Evolve** when your opponent plays a Unit.  
**Evolve**: Place a Plague Counter on and steal 3 stat points from each Unit.

**(1) Frontier Flora, Field Spell, Swarm, Common, Stinkyscape+, \#87**  
**Animated**  
**Evolves** when a **Starship** is launched.  
**Evolve**: Place 2 Plague Counters on this.  
**Aura**: Has 3X/3X stats for each Plague Counter.  
\~\~\~  
**Animated**  
**Evolves** when a **Starship** is launched.  
**Evolve**: Place 4 Plague Counters on this.  
**Aura**: Has 3X/3X stats for each Plague Counter.

**(2) Colony Watcher, Unit, Swarm, Common,** **Stinkyscape+, \#88**  
2/7  
**Evolve** when this moves from defense position to attack position.  
**Evolve**: Recruit a Swarm card.  
\~\~\~  
4/14  
**Evolve** when this moves from defense position to attack position.  
**Evolve**: Recruit 4 Swarm cards.

**(4) The Enigmata, Spell, Terminus, Mythic, Stinkyscape+, \#Token**  
Players take turns casting **Jogg’s** box until the game ends.  
\~\~\~  
Your hero gains **Armor 5** then Players take turns casting **Jogg’s** box until the game ends.

**(2) Animosity, Spell, Terminus, Mythic, Stinkyscape+, \#89**  
**Always starts on the bottom of your deck** *{if multiple cards have this order is random}*  
When your exile has 5 cards cast this *{wherever in your deck this is}*  
Add a random **Terminus** card to your hand.  
\~\~\~  
**Always starts on the bottom of your deck** *{if multiple cards have this order is random}*  
When your exile has 5 cards cast this *{wherever in your deck this is}*  
Add a random **Radiant Terminus** card to your hand.

**(3) Meltdown**, **Unit, Mindustrial, Legendary, Stinkyscape+, \#90**  
2/12  
**Flexible**  
**Ranged**  
**Can’t Defend**  
**Windfury**  
When this attack, gain \+2 attack and take 2 damage.  
When this is Frozen, fully heal it.  
\~\~\~  
4/28  
**Flexible**  
**Ranged**  
**Windfury**  
When this attack, gain \+4 attack and take 2 damage.  
When this is Frozen, fully heal it.

**(2) Tsunami**, **Unit, Mindustrial, Rare, Stinkyscape+, \#91**  
3/6  
**Flexible**  
**Ranged**  
**Can’t Defend**  
Freeze any Unit damaged by this.  
\~\~\~  
6/12  
**Flexible**  
**Ranged**  
**Can’t Defend**  
**Cleave**  
Freeze any Unit damaged by this.

**(1) Crystalline Chiton, Unit, Swarm, Common, Stinkyscape+, \#92**  
2/4  
**Evolves** when this survives damage.  
**Evolve**: Gain 2 Jade.  
\~\~\~  
4/8  
**Evolves** when this survives damage.  
**Evolve**: Gain 4 Jade.

**(2) Recessive Replicators,** **Unit, Swarm, Rare, Stinkyscape+, \#93**  
4/4  
**Evolve** when a friendly Unit dies.  
**Evolve**: Gain “Death: Resummon that Unit.”  
\~\~\~  
8/8  
**Evolve** when a friendly Unit dies.  
**Evolve**: Gain “Death: Resummon that Unit. Make it Radiant.”

**(3) Accelerationist,** **Unit, Human, Swarm, Common, Stinkyscape+, \#92**  
6/6  
**Aura**: Has \+2/+2 for every card that **Evolved** this game.  
\~\~\~  
12/12  
**Aura**: Has \+4/+4 for every card that **Evolved** this game.

**(2) KPIs, Spell, Epic, Stinkyscape+, \#93**  
Destroy every card with a lower winrate than this.  
\~\~\~  
Destroy every enemy card with a lower winrate than this.

**(1) PIP, Spell, Rare, Stinkyscape+, \#94**  
Put a Unit on a performance enhancement plan, they have 1 turn to get a kill otherwise they are destroyed.  
\~\~\~  
Put a Unit on a performance enhancement plan, they have 1 turn to get a kill otherwise they are destroyed. If they fail, add 3 **The Coins** to your hand.

**(3) JC Backed, Spell, Legendary, Stinkyscape+, \#95**  
Fill your hand with random S tier {or A tier if no S exist} cards. Summon 2 **Jlarna** and lose 1 mana for the rest of the game.  
\~\~\~  
Fill your hand with random S tier {or A tier if no S exist} cards. Summon 2 Radiant **Jlarna** for the rest of the game.

**(2) Open Enrollment, Spell, Common, Stinkyscape+, \#96**  
Fill your hand with random unique F tier {or D tier if no F exist} cards.  
\~\~\~  
Fill your hand with random unique Radiant F tier {or D tier if no F exist} cards.

**(4) Selective Enrollment, Spell, Common, Stinkyscape+, \#97**  
Fill your hand with random unique Radiant S tier {or A tier if no S exist} cards.  
\~\~\~  
Fill your hand with random unique Radiant S tier {or A tier if no S exist} cards, they cost (2) less.

**(2) Doomed Ship, Spell, Legendary, Stinkyscape+, \#98**  
Add a **Kwzki** and **Shockwave447**, **Shadiris** and **xxFreshJack**, or **Tylannt** and **Siva 1** to the left and right most cards of your hand. When they meet both become Radiant.  
\~\~\~  
Add a **Kwzki** and **Shockwave447**, **Shadiris** and **xxFreshJack**, or **Tylannt** and **Siva 1** to the left and right most cards of your hand. When they meet both become Radiant and cost (0).

**(1) Big Time Recruits, Spell, Epic, Stinkyscape+, \#99**  
Draw the 3 most expensive permanents and make them GIGA.  
\~\~\~  
Draw the 3 most expensive permanents and make them GIGA and Radiant.

**(2) Plesiosize, Spell, Rare, Stinkyscape+, \#100**  
Give a unit in your hand \+6/+6 and make it GIGA.  
\~\~\~  
Give a unit in your hand \+12/+12 and make it GIGA.

**(1) A Wee Whelp, Unit, Swarm, Common, Stinkyscape+, \#101**  
1/1  
**Evolves** after this destroys a Unit.  
**Evolve**: set stats to 10/10.  
\~\~\~  
2/2  
**Evolves** after this destroys a Unit.  
**Evolve**: set stats to 20/20.

**(1) Cosmic Bard, Unit, Rare, Stinkyscape+, \#102**  
3/2  
**Combo**: Draw 2 Units.  
\~\~\~  
6/4  
**Combo**: Draw 2 Units and set both their costs to the cheapest drawn.

**(3) Shadiris, Unit, Epic, Stinkyscape+, \#102**  
3/13  
**Cry:** Cast **Shadiris Visions** twice & add three random **Stinkscape Resources** to your hand.  
\~\~\~  
6/26  
**Cry:** Cast **Radiant Shadiris Visions** twice & add three random **Radiant Stinkscape Resources** to your hand.

**(1) Transcend, Spell, Common, Stinkyscape+, \#103**  
Transform your permanents into ones that cost (1) more.  
\~\~\~  
Transform your permanents into ones that cost (2) more.

**(1) Tireless Cultivation, Rare, Stinkyscape+, \#104**  
**Temporary 1**  
Transform a permanent into one that costs (1) more.  
**End of Turn**: Return this to your hand.  
\~\~\~  
**Temporary 2**  
Transform a permanent into one that costs (2) more.  
**End of Turn**: Return this to your hand.

**(3) Spacersi, Unit, Common, Stinkyscape+, \#105**  
5/8  
**Cry**: Transform your other Units into ones that cost (1) more.  
\~\~\~  
10/16  
**Cry**: Transform your other Units into ones that cost (2) more.

**(4) Spectre**, **Unit, Mindustrial, Legendary, Stinkyscape+, \#106**  
3/12  
**Flexible**  
**Indestructible**  
**GIGA**  
**Windfury**  
**Ranged**  
**Pierce**  
\~\~\~  
6/24  
**Flexible**  
**Indestructible**  
**GIGA**  
**Windfury**  
**Ranged**  
**Pierce**

**(2) Fuse**, **Unit, Mindustrial, Epic, Stinkyscape+, \#107**  
2/5  
**Armor 5**  
**Can’t Defend**  
**Ranged**  
Deal 5x damage to units in this lane, deals 3x damage to units in adjacent lanes  
**Pierce**  
\~\~\~  
4/10  
**Armor 10**  
**Can’t Defend**  
**Ranged**  
Deal 7x damage to units in this lane, deals 4x damage to units in adjacent lanes  
**Pierce**

**(2) Swarmer**, **Unit, Mindustrial, Rare, Stinkyscape+, \#108**  
2/4  
**Armor 1**  
**Can’t Defend**  
**Ranged**  
**Cleave**  
**Flexible**  
\~\~\~  
4/8  
**Armor 1**  
**Can’t Defend**  
**Ranged**  
**Cleave**  
**Flexible**  
**Pierce**

**(2) Lancer**, **Unit, Mindustrial, Rare, Stinkyscape+, \#109**  
2/4  
**Armor 2**  
**Can’t Attack**  
**Can’t Defend**  
**Flexible**  
**Activate 1: Discover** an enemy tile *{backrow lane 2, unit row lane 4, etc}* deal 5 **Pierce** deal damage to all units between that row and this *{draw a line KY’s papaya style}*  
\~\~\~  
4/8  
**Armor 4**  
**Can’t Attack**  
**Can’t Defend**  
**Flexible**  
**Activate 2: Discover** an enemy tile *{backrow lane 2, unit row lane 4, etc}* deal 8 **Pierce** deal damage to all units between that row and this *{draw a line KY’s papaya style}*

**(0) Dagger, Unit, Mindustrial, Automa-α, Common, Stinkyscape+, \#110**  
2/1  
**Immutable**  
**Immune to spells**  
**Ranged**  
**\~\~\~**  
4/2  
**Immutable**  
**Immune to spells**  
**Rush**  
**Ranged**

**(1) Mace, Unit, Mindustrial, Automa-α, Common, Stinkyscape+, \#111**  
2/2  
**Immutable**  
**Immune to spells**  
**Ranged**  
**Pierce**  
Damage also **Burns 1**  
**\~\~\~**  
4/4  
**Immutable**  
**Immune to spells**  
**Ranged**  
**Rush**  
**Pierce**  
Damage also **Burns 2**

**(2) Fortress, Unit, Mindustrial, Automa-α, Rare, Stinkyscape+, \#112**  
6/3  
**Tribute 1**  
**Immutable**  
**Immune to spells**  
**Ranged**  
**Pierce**  
**Armor 2**  
**\~\~\~**  
12/6  
**Tribute 1**  
**Immutable**  
**Immune to spells**  
**Ranged**  
**Pierce**  
**Rush**  
**Armor 4**

**(3) Scepter, Unit, Mindustrial, Automa-α, Automa-δ, Epic, Stinkyscape+, \#113**  
8/5  
**Tribute 2**  
**Immutable**  
**Immune to spells**  
**Ranged**  
**Windfury**  
**Pierce**  
**Armor 3**  
**\~\~\~**  
16/10  
**Tribute 2**  
**Immutable**  
**Immune to spells**  
**Ranged**  
**Windfury**  
**Pierce**  
**Rush**  
**Armor 3**

**(4) Reign, Unit, Mindustrial, Automa-α, Automa-Ω, Legendary, Stinkyscape+, \#114**  
12/8  
**Tribute 3**  
**Immutable**  
**Immune to spells**  
**Ranged**  
**Windfury**  
**Pierce**  
**Cleave**  
**Armor 4**  
Damage also **Burns 4**  
**\~\~\~**  
24/16  
**Tribute 3**  
**Immutable**  
**Immune to spells**  
**Ranged**  
**Windfury**  
**Pierce**  
**Rush**  
**Cleave**  
**Armor 4**  
Damage also **Burns 8**

**(0) Flare, Unit, Mindustrial, Automa-β, Common, Stinkyscape+, \#115**  
1/1  
**Flying**  
**Ranged**  
**Rush**  
**Can’t Defend**  
\~\~\~  
2/2  
**Flying**  
**Ranged**  
**Rush**  
**Reborn**  
**Can’t Defend**

**(1) Horizon, Unit, Mindustrial, Automa-β, Common, Stinkyscape+, \#116**  
3/1  
**Flying**  
**Ranged**  
**Rush**  
**Can’t Defend**  
\~\~\~  
6/2  
**Flying**  
**Ranged**  
**Rush**  
**Reborn**  
**Can’t Defend**

**(2) Zenith, Unit, Mindustrial, Automa-β, Rare, Stinkyscape+, \#117**  
5/3  
**Tribute 1**  
**Flying**  
**Ranged**  
**Pierce**  
Damage also**Burns 3**  
**Can’t Defend**  
\~\~\~  
10/6  
**Tribute 1**  
**Flying**  
**Ranged**  
**Pierce**  
**Rush**  
Damage also **Burns 6**  
**Can’t Defend**

**(3) Antumbra, Unit, Mindustrial, Automa-β, Automa-δ, Epic, Stinkyscape+, \#118**  
6/13  
**Tribute 2**  
**Flying**  
**Ranged**  
**Pierce**  
Damage also **Burns 4**  
**Can’t Defend**  
\~\~\~  
12/26  
**Tribute 2**  
**Flying**  
**Ranged**  
**Pierce**  
**Rush**  
Damage also **Burns 8**  
**Can’t Defend**

**(4) Eclipse, Unit, Mindustrial, Automa-β, Automa-Ω, Legendary, Stinkyscape+, \#119**  
7/15  
**Tribute 3**  
**Flying**  
**Ranged**  
**Pierce**  
Dealing damage **Burns 5**  
**Can’t Defend**  
**Crater**  
**Armor 3**  
\~\~\~  
14/30  
**Tribute 3**  
**Flying**  
**Ranged**  
**Pierce**  
**Rush**  
Dealing damage **Burns 10**  
**Can’t Defend**  
**Crater**  
**Armor 3**

**(0) Crawler, Unit, Mindustrial, Automa-γ, Common, Stinkyscape+, \#120**  
The first this game that this enter your board, add a **Toxopid** to your hand.  
May be played from your GY for (1)  
1/3  
**Can’t defend**  
After this attack DIE  
**Death:** deal 2 damage to the last unit this damaged  
\~\~\~  
The first this game that this enter your board, add a **Radiant Toxopid** to your hand.   
May be played from your GY for (1)  
2/6  
**Can’t defend**  
After this attack DIE  
**Death:** deal 4 damage to the last unit this damaged

**(10) Toxopid, Unit, Mindustrial, Automa-γ, Mythic, Stinkyscape+, \#121**  
The first this game that this enters your hand, add a **Crawler** to your hand.  
**Float** reduces mana. Friendly **Crawlers Dying** reduces mana cost by (1)  
**Cry:** Summon 3 **Toxopid Legs** ANYWHERE on the field *{exiling the card below it, should trigger leg’s crater}*  
**Crater**  
**Immutable**  
**Indestructible** while **Toxopid Legs** are on the field  
10/10  
**Rush**  
**Ignores Taunt**  
**End of turn: Toxic 2** all enemies  
\~\~\~  
The first this game that this enters your hand, add a **Crawler** to your hand.  
**Float** reduces mana. Friendly **Crawlers Dying** reduces mana cost by (1)  
**Cry:** Summon 4 **Radiant Toxopid Legs** on your opponents field *{exiling the card below it, should trigger leg’s crater}*  
**Crater**  
**Immutable**  
**Indestructible** while **Toxopid Legs** are on the field  
20/20  
**Rush**  
**Ignores Taunt**  
**End of turn: Toxic 4** all enemies

**(3) Toxopid Leg, Unit, Mindustrial, Token, Automa-γ, Mythic, Stinkyscape+, \#121.1**  
2/8  
**Crater**  
**Can’t attack**  
**Taunt**  
\~\~\~  
4/16  
**Crater**  
**Can’t attack**  
**Taunt**

**(3) Quad, Unit, Mindustrial, Automa-δ, Epic, Stinkyscape+, \#122**  
3/12  
**Tribute 2**  
**Can’t attack**   
**Can’t defend**  
**Flying**  
**End of turn:** Heal all allies 6 hp and deal 3 damage to all enemies  
\~\~\~  
6/24  
**Tribute 2**  
**Can’t attack**   
**Can’t defend**  
**Flying**  
**Armor 2**  
**End of turn:** Heal all allies 12 hp and deal 6 damage to all enemies

**(4) Oct, Unit, Mindustrial, Automa-Ω, Legendary, Stinkyscape+, \#123**  
4/16  
**Tribute 3**  
**Can’t attack**   
**Can’t defend**  
**Flying**  
**End of turn:** Heal all allies 8 hp   
**Aura:** The first 5 damage allies would take per turn is nullified *{5 left}*  
\~\~\~  
8/32  
**Tribute 3**  
**Can’t attack**   
**Can’t defend**  
**Flying**  
**End of turn:** Heal all allies 16 hp   
**Armor 2**  
The first 10 damage allies would take per turn is nullified *{10 left}*

**(3) Sei, Unit, Mindustrial, Automa-δ, Epic, Stinkyscape+, \#124**  
5/12  
**Tribute 2**  
**Ranged**  
**Taunt**  
**Armor 5**  
**Activate 1:** Deal 2 **Pierce Cleave** damage  
\~\~\~  
10/24  
**Tribute 2**  
**Ranged**  
**Taunt**  
**Armor 10**  
**Activate 2:** Deal 2 **Pierce Cleave** damage

**(4) Omura, Unit, Mindustrial, Automa-Ω, Legendary, Stinkyscape+, \#125**  
7/17  
**Tribute 3**  
**Ranged**  
**Taunt**  
**Armor 8**  
**Activate 1:** Deal 12 **Pierce Trample** damage *{has a three turn cooldown, e.g., normal activates have a 1 turn cooldown}*  
\~\~\~  
14/34  
**Tribute 3**  
**Ranged**  
**Taunt**  
**Armor 16**  
**Activate 1:** Deal 24 **Pierce Trample** damage *{has a three turn cooldown, e.g., normal activates have a 1 turn cooldown}*

**(3) Vela, Unit, Mindustrial, Automa-δ, Epic, Stinkyscape+, \#126**  
0/10  
**Can’t attack, Can’t defend**  
**Tribute 2**  
**Activate 1: Choose** an enemy tile *{backrow lane 2, unit row lane 4, etc}* deal 8 **Pierce** deal damage to all units between that row and this *{draw a line KY’s papaya style}*  
\~\~\~  
0/20  
**Can’t attack, Can’t defend**  
**Tribute 2**  
**Activate 1: Choose** an enemy tile *{backrow lane 2, unit row lane 4, etc}* deal 8 **Cleave Pierce** deal damage to all units between that row and this *{draw a line KY’s papaya style}*

**(4) Corvus, Unit, Mindustrial, Automa-Ω, Legendary, Stinkyscape+, \#127**  
0/20  
**Can’t attack, Can’t defend**  
**Tribute 3**  
**Activate 1: Choose** an enemy tile *{backrow lane 2, unit row lane 4, etc}* deal 80 **Pierce** deal damage to all units between that row and this *{draw a line KY’s papaya style} {animation should be of a laser}*  
\~\~\~  
0/40  
**Can’t attack, Can’t defend**  
**Tribute 3**  
**Activate 1: Choose** an enemy tile *{backrow lane 2, unit row lane 4, etc}* deal 80 **Cleave Pierce** deal damage to all units between that row and this *{draw a line KY’s papaya style} {animation should be of a laser}*

**(1) Additive Reconstructor, Fieldspell, Mindustrial, Rare, Stinkyscape+, \#128**  
**Start of your turn**, add a random (0) or (1) cost **Automa** card to your hand  
\~\~\~  
**Start of your turn**, add a random **Radiant** (0) or (1) cost **Automa** card to your hand

**(3) Multiplicative Reconstructor, Fieldspell, Mindustrial, Rare, Stinkyscape+, \#129**  
**Start of your turn**, add a random (2) cost **Automa** card to your hand, it has no **Tribute** condition  
\~\~\~  
**Start of your turn**, add a random **Radiant** (2) cost **Automa** card to your hand, it has no **Tribute** condition

**(4) Exponential Reconstructor, Fieldspell, Mindustrial, Epic, Stinkyscape+, \#130**  
**Tribute 1**  
**Start of your turn**, add a random **Automa-δ** card to your hand, it has no **Tribute** condition  
\~\~\~  
**Tribute 1**  
**Start of your turn**, add a random **Radiant** **Automa-δ** card to your hand, it has no **Tribute** condition

**(4) Tetrative Reconstructor, Fieldspell, Mindustrial, Legendary, Stinkyscape+, \#131**  
**Tribute 4**  
**Start of your turn**, add a random **Automa-Ω** card to your hand, it has no **Tribute** condition  
\~\~\~  
**Tribute 4**  
**Start of your turn**, add a random **Radiant** **Automa-Ω** card to your hand, it has no **Tribute** condition

**(2) Great Paperclip Theory II, Paperclip, Quickdraw, Spell, Mindustrial, Mythic, Stinkyscape+, \#132**  
**Choose one** α or β**;**   
Add a **Dagger/Flare** to your hand. The first time you play each **Automa-α/β** cost card add the subsequent cost **Automa-α/β** to your hand.   
\~\~\~  
**Choose one** α or β**;**   
Add a **Radiant** **Dagger/Flare** to your hand. The first time you play each **Automa-α/β** cost card add the subsequent cost **Radiant** **Automa-α/β** to your hand. 

**(4) Great Paperclip Theory III, Paperclip, Spell, Mindustrial, Mythic, Stinkyscape+, \#133**  
Add a **Daglare** to your hand. The first time you play each **Automa-ζ** cost card add the subsequent cost **Automa-ζ** to your hand.   
\~\~\~  
Add a **Radiant** **Dagger & Flare** to your hand. The first time you play each **Radiant** **Automa-ζ** cost card add the subsequent cost **Radiant** **Automa-ζ** to your hand. 

**(0) Daglare, Unit, Mindustrial, Token, Automa-ζ, Common, Stinkyscape+, \#133.1**  
*Fusion of Dagger \+ Flare; unique art & name, cost (0)*

**(1) Macizon, Unit, Mindustrial, Token, Automa-ζ, Common, Stinkyscape+, \#133.2**  
*Fusion of Mace \+ Horizon; unique art & name, cost (1)*

**(2) Fortnith, Unit, Mindustrial, Token, Automa-ζ, Rare, Stinkyscape+, \#133.3**  
*Fusion of Fortress \+ Zenith; unique art & name, cost (2), Tribute 1*

**(3) Sceptumbra, Unit, Mindustrial, Token, Automa-ζ, Automa-δ, Epic, Stinkyscape+, \#133.4**  
*Fusion of Scepter \+ Antumbra; unique art & name, cost (3), Tribute 2*

**(4) Reignclipse, Unit, Mindustrial, Token, Automa-ζ, Automa-Ω, Legendary, Stinkyscape+, \#133.5**  
*Fusion of Regin \+ Eclipse; unique art & name, cost (4), Tribute 4*

**(2) Basket of Crawlers, Spell, Mindustrial, Common, Stinkyscape+, \#134**  
Add 3 **Crawlers** to your hand  
\~\~\~  
Add 3 **Radiant** **Crawlers** to your hand

**(2) Crawler Swarm, Spell, Mindustrial, Common, Stinkyscape+, \#135**  
Fill your board with **Crawlers**   
\~\~\~  
Fill your board with **Radiant Crawlers**

**(2) Crawler Assembly Line, Field Spell, Mindustrial, Rare, Stinkyscape+, \#136**  
**Activate 1:** Summon a **Crawler**  
\~\~\~  
**Activate 1:** Summon a **Radiant** **Crawler** 

**(1) The Perfect Pair, Spell, Mindustrial, Rare, Stinkyscape+, \#137**  
Add a **Automa-δ** and its paired **Automa-Ω** card to your hand  
*{pairs are: Scepter & Reign, Antumbra & Eclipse, Quad & Oct, Sei & Omura, Vela & Corvus}*  
\~\~\~  
Add a **Radiant** **Automa-δ** and its paired **Radiant** **Automa-Ω** card to your hand

**(2 embiggen 4\) Delta Definition, Spell, Mindustrial, Epic, Stinkyscape+, \#138**  
Add 3 **Automa-δ** cards to your hand, embiggen they have Tribute 1  
\~\~\~  
Add 3 **Radiant Automa-δ** cards to your hand, embiggen they have Tribute 1

**(2 embiggen 4\) Operation Omega, Spell, Mindustrial, Legendary, Stinkyscape+, \#139**  
Add 2 **Automa-Ω** cards to your hand, embiggen they have Tribute 2  
\~\~\~  
Add 2 **Radiant Automa-Ω** cards to your hand, embiggen they have Tribute 2

**(1) Automa Reintegration Plan, Spell, Mindustrial, Common, Stinkyscape+, \#140**  
Discover a **Automa** unit, it gains **Starship Piece**   
\~\~\~  
Discover a **Radiant Automa** unit, it gains **Starship Piece** 

**(2) Mindustrial Revolution, Spell, Mindustrial, Rare, Stinkyscape+, \#140**  
For the rest of the game at the start of your turn add a random **Mindustrial** card to your hand  
\~\~\~  
For the rest of the game at the start of your turn add a random **Radiant Mindustrial** card to your hand, it costs (1) less

**(1) Stinky 1x Engineer, Unit, Human, Common, Stinkyscape+, \#141**  
2/3  
**Cry**: Nerf 3 other random permanents.  
{For AI: Voiceline is “Prod is down. Report the intern.”}  
\~\~\~  
4/6  
**Cry**: Destroy 3 other random permanents.

**(4) Call to Chaos (Mindustrial-Swarm Edition) Spell, Call to Chaos, Stinkyscape+, Legendary, \#142**  
???  
*(One of the following random effects)*

- Add 3 random **Paperclip** cards to your hand, they cost (0), **Refresh 2 mana**   
- Add 3 random **Evolved Swarm** cards to your hand, they cost (0)   
- Summon two random **Radiant** units, make them **GIGA**  
- Cast 5 **Jlockheed** cards  
- Add 3 **Terminus** cards to your hand, **Refresh 4 mana**  
- Add the **Toxopid** to your hand, it costs (0)  
- **Toxic 3** and **Burn 3** all enemies   
- For the rest of the game, when you **Launch** a **starship** cast a random **Call to Chaos**, summon a random (2) cost **Weapon**  
- Summon a **Mindustrial Golem** and a **Swarm Golem**  
- Cast a random **Call to Chaos**

\~\~\~  
\!\!\!  
*(three effects)*

**(4) Mindustrial Golem Unit, Token, Mindustrial, Stinkscape+, Legendary, \#142.1**  
5/5  
**Rush**  
**Ranged**  
After this attacks summon a **Random Mindustrial card** that **can’t attack**  
\~\~\~   
10/10  
**Rush**  
**Ranged**  
After this attacks summon a **Random Radiant Mindustrial card** that **can’t attack**

**(4) Swarm Golem Unit, Token, Mindustrial, Stinkscape+, Legendary, \#142.2**  
5/5  
**Rush**  
After this attacks summon a **Swarm** card that is **Evolved**  
\~\~\~   
10/10  
**Rush**  
After this attacks summon a **Radiant Swarm** card that is **Evolved**

**(2) Fire Beam v3, Unit, Rare, Weapon, Stinkyscape+, \#143**  
\[0/2\]  
**Starship Piece**  
**Equip**  
**Activate 1: Burn 2** an enemy unit, **Cleave**  
\~\~\~  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1: Burn 4** an enemy unit, **Cleave**

**(3) Fire Beam v4, Unit, Epic, Weapon, Stinkyscape+, \#144**  
\[0/3\]  
**Starship Piece**  
**Equip**  
**Activate 1: Burn 4** an enemy unit, **Cleave**  
\~\~\~  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 1: Burn 8** an enemy unit, **Cleave**

**(4) Fire Beam v5, Unit, Legendary, Weapon, Stinkyscape+, \#145**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 2: Burn 4** an enemy unit, **Cleave**  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 2: Burn 8** an enemy unit, **Cleave**

**(2) Scourge Mortar v3, Unit, Rare, Weapon, Stinkyscape+, \#146**  
\[0/2\]  
**Starship Piece**  
**Equip**  
**Activate 1: Toxic 2** an enemy unit  
\~\~\~  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 2: Toxic 2** an enemy unit

**(3) Scourge Mortar v4, Unit, Epic, Weapon, Stinkyscape+, \#147**  
\[0/3\]  
**Starship Piece**  
**Equip**  
**Activate 1: Toxic 4** an enemy unit  
\~\~\~  
\[0/6\]  
**Starship Piece**  
**Equip**  
**Activate 2: Toxic 4** an enemy unit

**(4) Scourge Mortar v5, Unit, Legendary, Weapon, Stinkyscape+, \#148**  
\[0/4\]  
**Starship Piece**  
**Equip**  
**Activate 1: Toxic 6** an enemy unit  
\~\~\~  
\[0/8\]  
**Starship Piece**  
**Equip**  
**Activate 2: Toxic 6** an enemy unit

**(1) Flaming Rounds, Unit, Common, Stinkyscape+, \#149**  
2/1  
**Starship Piece**  
**Equip**  
Dealing damage **Burns 2**  
\~\~\~  
4/2  
**Starship Piece**  
**Equip**  
Dealing damage **Burns 4**

**(2) Scorching Rounds, Unit, Rare, Stinkyscape+, \#150**  
3/1  
**Starship Piece**  
**Equip**  
Damage also **Burns 3**  
\~\~\~  
6/2  
**Starship Piece**  
**Equip**  
Damage also **Burns 6**

**(4) Volcanic Rounds, Unit, Epic, Stinkyscape+, \#151**  
3/1  
**Starship Piece**  
**Equip**  
Damage dealt **Burns**  
\~\~\~  
6/2  
**Starship Piece**  
**Equip**  
Damage dealt **Burns**

**(2) Venomous Rounds, Unit, Rare, Stinkyscape+, \#152**  
2/1  
**Starship Piece**  
**Equip**  
Damage also applies **Toxic 2**  
\~\~\~  
4/2  
**Starship Piece**  
**Equip**  
Damage also applies **Toxic 4**

**(4) Noxious Rounds, Unit, Epic, Stinkyscape+, \#153**  
1/1  
**Starship Piece**  
**Equip**  
Damage dealt is **Toxic**  
\~\~\~  
2/2  
**Starship Piece**  
**Equip**  
Damage dealt is **Toxic**

**(2) Chaos Accelerator, Unit, Epic, Stinkyscape+, \#154**  
3/6  
**Starship Piece**  
When launched, cast a random **Call to Chaos**  
\~\~\~  
6/12  
**Starship Piece**  
When launched, cast a random **Radiant Call to Chaos**

**(4) The Far Vagabond, Unit, Mythic, Stinkyscape+, \#155**  
Summon a random **Terminus Starships**, it can be launched for (3) *{each one should have 5\~12 mana worth of attachments, special names, different art than normal Starships}*  
\~\~\~  
Summon a random **Terminus Starships**, it can be launched for (0) *{each one should have 5\~12 mana worth of attachments, special names, different art than normal Starships}*

**(4) Horizon Breaker, Unit, Epic, Stinkyscape+, \#156**  
3/6  
**Starship Piece**  
When launched, cast a random **Terminus Card**  
\~\~\~  
6/12  
**Starship Piece**  
When launched, cast a random **Radiant Terminus Card**

**(4) Phantom Shipyard Unit, Legendary, Stinkyscape+, \#157**  
0/0  
**Start of turn: Fuse** a **random Starship Piece** into this  
**Dormant until launched**  
**Activate 3: Exile the shipyard and launch as a starship**  
*{for ai it should be similar to the Starship card where it summons a starship and such idk work on edge cases}*  
\~\~\~  
0/0  
**Start of turn: Fuse** a **random Radiant Starship Piece** into this  
**Dormant until launched**  
**Activate 3: Exile the shipyard and launch as a starship**

**(1) Quick Chaos, Spell, Rare, Stinkyscape+, \#158**  
**Cast on draw:** cast a random spell *{targets chosen randomly}*  
\~\~\~  
**Cast on draw:** cast a random **Radiant** spell *{targets chosen randomly}*

**(0) Raven, Unit, Rare, Stinkyscape+, \#159**  
\[1/1\]  
**Tribute 1**  
**Cry:** Discover 1 **Rare** **Weapons**; **Equip** them into this  
\~\~\~  
\[2/2\]  
**Tribute 1**  
**Cry:** Discover 1 **Epic** **Weapons**; **Equip** them into this

**(4) Icarus, Unit, Legendary, Stinkyscape+, \#160**  
**Tribute 3**  
\[2/8\]  
**Activate 1:** Summon a **Radi** *{trigger it’s cry}*  
**Cry:** Discover a **Epic** **Weapons**; **Equip** it into this  
\~\~\~  
**Tribute 3**  
\[4/16\]  
**Activate 1:** Summon a **Radiant** **Radi** *{trigger it’s cry}*  
**Cry:** Discover a **Legendary** **Weapons**; **Equip** it into this  
