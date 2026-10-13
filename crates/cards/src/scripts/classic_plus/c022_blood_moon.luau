--!strict
local J = require("@jackioh")

-- `{ id: "blood-moon", on: "healed", instead }`: the one replacement both faces declare. The base
-- face's conversion lasts the rest of the turn; the Radiant face is a Field Trap that keeps it.
local base = {
	replacements = {
		{
			id = "blood-moon",
			on = "healed",
			instead = { damage = "pierce", lasting = "thisTurn" },
		},
	},
}

local radiant = {
	staticFlags = { healToDamage = true },
	replacements = {
		{
			id = "blood-moon",
			on = "healed",
			instead = { damage = "pierce" },
		},
	},
}

return {
	base = base,
	radiant = radiant,
}
