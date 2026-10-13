--!strict
local J = require("@jackioh")

-- Lane-only attacks (§4.2 step 2) and never Defense: E35's static flags. The same script on both
-- faces: Armor 6 and Immune to Spells are the Radiant face's catalog keywords.
local base = {
	staticFlags = {
		attackedOnlyFromLane = true,
		neverDefense = true,
	},
}

return {
	base = base,
	radiant = "base",
}
