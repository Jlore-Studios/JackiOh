--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = 3 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
