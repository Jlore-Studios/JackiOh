--!strict
local J = require("@jackioh")

return {
	-- E10's `end_turn` (R456) resolves the rest of the list first, so the immunity is set and the
	-- turn then ends.
	base = {
		cry = function(ctx: J.EffectContext)
			return {
				J.end_turn({}),
				J.make_hero_immune({}),
			}
		end,
	},
	radiant = {
		cry = function(ctx: J.EffectContext)
			return {
				J.make_hero_immune({}),
			}
		end,
	},
}
