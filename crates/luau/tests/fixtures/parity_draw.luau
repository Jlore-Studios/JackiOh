--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.draw({ count = 2 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
