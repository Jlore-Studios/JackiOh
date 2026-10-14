--!strict
local J = require("@jackioh")

-- ME-STATS: the compiled win-rate table's leader, never CN Tech itself (R387); a random non-token
-- card when no row qualifies (R142).
local function tech(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local leader = ctx:win_rate_leader("meditative-050")
			if leader ~= nil then
				return {
					J.add_to_hand({ defId = leader, radiant = radiant }),
				}
			end
			return {
				J.add_random_from_catalog({ query = {}, radiant = radiant }),
			}
		end,
	}
end

return {
	base = tech(false),
	radiant = tech(true),
}
