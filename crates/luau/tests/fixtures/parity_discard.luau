--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.discard_random({ count = 1 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
