--!strict
local J = require("@jackioh")

-- Pierce: the damage ignores Armor (R346).
local function whirl(ctx: J.EffectContext)
	return {
		J.damage_all({
			amount = ctx:param("damage"),
			side = "any",
			ignoreArmor = true,
		}),
	}
end

local base = {
	cry = whirl,
}

-- The same Cry, and back from the graveyard to hand at the end of the turn it was played (R68). A
-- card that is nowhere hands back a `null` that is not nil, so the test is for a card.
local radiant = {
	cry = whirl,
	endOfTurn = function(ctx: J.EffectContext)
		local me = ctx:live_self()
		if type(me) == "table" and me.returnToHandAtEndOfTurn == true then
			return {
				J.bounce({ target = { of = "self" } }),
			}
		end
		return {}
	end,
}

return {
	base = base,
	radiant = radiant,
}
