--!strict
local J = require("@jackioh")

-- Start of turn: `units` summons of a random Unit from the hand (R1083), Radiant on the Radiant face.
local function skull(radiant: boolean)
	return {
		startOfTurn = function(ctx: J.EffectContext)
			local effects = {}
			for _ = 1, ctx:param("units") do
				effects[#effects + 1] = J.summon_random_from_hand({ radiant = radiant })
			end
			return effects
		end,
	}
end

return {
	base = skull(false),
	radiant = skull(true),
}
