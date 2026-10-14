--!strict
local J = require("@jackioh")

-- §3.2: "worth 2 Tributes while on the field"; §7's radiant face: "2/2, worth 3 Tributes". The
-- numbers are the engine's named constants (CLAUDE.md rule 9).
local base = {
	staticFlags = { tributeWorth = J.config.SHEEP_TRIBUTE_VALUE },
}

local radiant = {
	staticFlags = { tributeWorth = J.config.RADIANT_SHEEP_TRIBUTE_VALUE },
}

return {
	base = base,
	radiant = radiant,
}
