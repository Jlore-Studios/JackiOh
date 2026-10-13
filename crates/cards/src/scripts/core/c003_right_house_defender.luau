--!strict
local J = require("@jackioh")

local radiant = {
	death = function(ctx)
		return {
			J.summon({
				defId = "core-003",
				player = "self",
			}),
		}
	end,
}

return {
	base = {},
	radiant = radiant,
}
