--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.after_state_check(function(inner)
				return {
					J.damage({ to = { of = "enemyHero" }, amount = 2 }),
				}
			end),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
