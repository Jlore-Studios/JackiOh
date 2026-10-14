--!strict
local J = require("@jackioh")

-- ME-LETHALGUARD (R1204): §4.4 step 4a opens for Units too, so a fatal hit on a friendly ally goes
-- to Unan through Unan's own Armor, each Unan catching a given hit once.
local base = {
	replacements = {
		{
			id = "92-lethal-guard",
			on = "lethalHit",
			instead = { redirect = "self" },
		},
	},
}

-- Both faces catch the same way; the Radiant face's Armor is catalog data beside it.
return {
	base = base,
	radiant = "base",
}
