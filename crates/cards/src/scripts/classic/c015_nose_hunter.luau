--!strict
local J = require("@jackioh")

-- R392: an Activate ♾️ ability whose cost is the random discard, paid as it is activated
-- (`cost.discardRandom`), so an empty hand cannot activate it.
local COST = { discardRandom = 1 }

-- "Exile the bottom {exile} card(s) of your opponent's deck".
local function bottom(ctx: J.EffectContext): J.EffectCall
	return J.exile_bottom_of_library({ player = "enemy", count = ctx:param("exile") })
end

local base = {
	activations = {
		{
			id = "hunt",
			label = "Discard a random card. Exile the bottom of your opponent's deck",
			uses = "unlimited",
			cost = COST,
			run = function(ctx: J.EffectContext)
				return { bottom(ctx) }
			end,
		},
	},
}

-- "… and a random card from their hand" (R60; an empty hand: nothing).
local radiant = {
	activations = {
		{
			id = "hunt",
			label = "Discard a random card. Exile the bottom of your opponent's deck and a random card from their hand",
			uses = "unlimited",
			cost = COST,
			run = function(ctx: J.EffectContext)
				return {
					bottom(ctx),
					J.exile_random_from_hand({ player = "enemy", count = 1 }),
				}
			end,
		},
	},
}

return {
	base = base,
	radiant = radiant,
}
