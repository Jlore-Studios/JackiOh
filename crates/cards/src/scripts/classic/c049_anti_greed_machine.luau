--!strict
local J = require("@jackioh")

-- "Players can't draw more than 1 card each turn": the base face's printed limit, on both players.
local PLAYERS_LIMIT = 1

return {
	base = {
		drawLimit = function(ctx: J.HookArgs)
			local limit: J.DrawLimit = { player = "both", count = PLAYERS_LIMIT }
			return { limit }
		end,
	},
	-- "Your opponent can't draw more than {limit} …": the declared, tunable number.
	radiant = {
		drawLimit = function(ctx: J.HookArgs)
			local limit: J.DrawLimit = { player = "enemy", count = ctx:param("limit") }
			return { limit }
		end,
	},
}
