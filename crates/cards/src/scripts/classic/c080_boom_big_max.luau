--!strict
local J = require("@jackioh")

-- §6.3 "Tribute 3": three of your Units, or one Sheep Token plus one more, worth 3 (§3.2).
local TRIBUTE_COST = 3

-- The same script on both faces: the Radiant face's 16 health and Charge are catalog data, and it
-- keeps Tribute 3.
local base = {
	staticFlags = { tribute = TRIBUTE_COST },
}

return {
	base = base,
	radiant = "base",
}
