--!strict
local J = require("@jackioh")

local function lastHurrah(turn: J.DiscardHandTurn)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.draw({ count = ctx:zone_count(ctx:controller(), "library") }),
				J.discard_hand_at_turn_end({ turn = turn }),
			}
		end,
	}
end

return {
	base = lastHurrah("this"),
	radiant = lastHurrah("next"),
}
