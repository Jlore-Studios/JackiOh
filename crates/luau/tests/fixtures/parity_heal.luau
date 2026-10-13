--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.damage({ to = { of = "selfHero" }, amount = 5 }),
			J.heal({ target = { of = "selfHero" }, amount = 3 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
