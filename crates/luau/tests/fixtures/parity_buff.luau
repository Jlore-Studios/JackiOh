--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.buff_all_units({ attack = 1, health = 1 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
