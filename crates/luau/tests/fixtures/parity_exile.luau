--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.exile_all({ side = "enemy" }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
