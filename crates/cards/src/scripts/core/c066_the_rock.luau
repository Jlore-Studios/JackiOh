--!strict
local J = require("@jackioh")

-- §6.3 "Tribute 1": one of your units, or one Sheep Token, which is worth 2 of it (§3.2).
local TRIBUTE_COST = 1

-- The radiant cell adds a keyword and nothing else, and §8's Conventions keep every base clause a
-- radiant cell does not restate — so the Tribute cost is kept and the face is the same.
local base = {
	staticFlags = { tribute = TRIBUTE_COST },
}

return {
	base = base,
	radiant = "base",
}
