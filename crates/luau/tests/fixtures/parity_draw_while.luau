--!strict
local J = require("@jackioh")

-- Draw while fewer than three cards are in hand.
local base = {
	cry = function(ctx)
		return {
			J.draw_while(function(inner)
				return #inner:zone_cards(inner:controller(), "hand") < 3
			end),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
