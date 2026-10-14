--!strict
local J = require("@jackioh")

-- Core #5 Stockpile.
local STOCKPILE = "core-005"

-- "Death: Add {cards} Stockpiles to your hand. Each costs (0)": the declared numbers `cards` and
-- `setCost` (R386). The Radiant face's Stockpiles are Radiant.
local function bauble_bubble(ctx: J.EffectContext, radiant: boolean)
	local cost = ctx:param("setCost")
	local effects = {}
	for _ = 1, ctx:param("cards") do
		effects[#effects + 1] = J.add_to_hand({ defId = STOCKPILE, costOverride = cost, radiant = radiant })
	end
	return effects
end

local base = {
	death = function(ctx: J.EffectContext)
		return bauble_bubble(ctx, false)
	end,
}

local radiant = {
	death = function(ctx: J.EffectContext)
		return bauble_bubble(ctx, true)
	end,
}

return {
	base = base,
	radiant = radiant,
}
