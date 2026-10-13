--!strict
local J = require("@jackioh")

-- MD-D31, R1124: whatever this damages is exiled at the next state check. The Radiant face's Cleave
-- is catalog data, so the faces share the flag.
local base = {
	staticFlags = { exilesOnDamage = true },
}

return {
	base = base,
	radiant = "base",
}
