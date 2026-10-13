--!strict
local J = require("@jackioh")

-- §8: "Tribute 3", counted with the Sheep Token worth 2 (§3.2).
local TRIBUTE_COST = 3

-- The Radiant face: its Tribute, and the permission to pay it with opposing units (R101).
local radiant = {
	staticFlags = {
		tribute = TRIBUTE_COST,
		tributeEnemies = true,
	},
}

-- The base face adds its price: paid with an opposing unit, it is summoned for the opponent (R360).
local base = {
	staticFlags = {
		tribute = TRIBUTE_COST,
		tributeEnemies = true,
		enemyTributeHandsOver = true,
	},
}

return {
	base = base,
	radiant = radiant,
}
