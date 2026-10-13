--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.gain_mana({ amount = 3 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
