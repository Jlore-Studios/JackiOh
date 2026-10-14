--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		return {
			J.summon({ defId = "lx-fx-001" }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
