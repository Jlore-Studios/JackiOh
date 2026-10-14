-- C #35 Prep (SPEC §8.6 row 35). (0) Spell, Common.
--   Base:    "Your next Spell this turn costs ({discount}) less." — discount 2
--   Radiant: "Your next Spell this turn costs ({discount}) less." — discount 4
--
-- Core #35 Lunar Eclipse's player modifier: `onlyType: "Spell"` (a Unit, Field Spell or
-- Trap play neither pays less nor spends it), `oncePerTurn` (§10.5 step 2 removes it on the
-- first Spell it priced) and `{ until: "thisTurn" }` (§2.2's cleanup takes it unused). R65
-- floors the price at (0). R70: a cast pays nothing and never uses a discount, so it is left
-- for the next Spell played. The amount is the declared `discount` (R386), read through
-- `param`; both faces run this script.
local J = require("@jackioh")

local function cry(ctx)
	local t = ctx:turn()
	return {
		J.add_player_modifier({
			player = "self",
			mod = {
				kind = "costDiscount",
				amount = ctx:param("discount"),
				onlyType = "Spell",
				oncePerTurn = true,
				expiry = { ["until"] = "thisTurn", turn = t.number },
			},
		}),
	}
end

-- The same script: the Radiant face's 4 is its declared `discount`, which `param` reads
-- off the running face.
return { base = { cry = cry }, radiant = { cry = cry } }
