--!strict
local J = require("@jackioh")

-- Draw while fewer than three cards are in hand.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.draw_while({ hook = "draw_more" }),
		}
	end,
	draw_more = function(ctx: J.EffectContext)
		return #ctx:zone_cards(ctx:controller(), "hand") < 3
	end,
}

return {
	base = base,
	radiant = "base",
}
