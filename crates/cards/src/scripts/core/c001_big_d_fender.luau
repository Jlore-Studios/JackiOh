--!strict
local J = require("@jackioh")

local base = {
	aura = function(ctx)
		return {
			J.defense_armor_aura({ armor = ctx:param("armor") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
