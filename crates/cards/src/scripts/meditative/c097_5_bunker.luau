--!strict

-- §8.8: "Tribute 2".
local TRIBUTE_COST = 2

-- Multiplies attack by 3 when striking across its own lane (LANE-STRIKE, MD-F14).
local MULTIPLIER = 3

local base = {
	staticFlags = {
		tribute = TRIBUTE_COST,
		laneMultiplier = MULTIPLIER,
	},
}

return {
	base = base,
	radiant = "base",
}
