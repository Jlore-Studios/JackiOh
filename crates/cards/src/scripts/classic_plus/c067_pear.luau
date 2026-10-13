--!strict
local J = require("@jackioh")

-- §8.7: the printed cost of the Units it summons.
local UNIT_COST = 1

local function pear(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local effects = {}
			for _ = 1, ctx:param("units") do
				effects[#effects + 1] = J.summon_random({
					query = { type = "Unit", cost = UNIT_COST, rarity = "Common" },
					radiant = if radiant then true else nil,
				})
			end
			return effects
		end,
	}
end

return {
	base = pear(false),
	radiant = pear(true),
}
