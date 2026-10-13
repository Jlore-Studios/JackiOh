--!strict
local J = require("@jackioh")

-- On the Radiant face, the Unit is Buffed (Upgraded) once as it lands.
local BUFFS = 1

-- STACK-BASE (MD-F12): your played Units may top this as if they had Stack.
local base = {
	staticFlags = { stackBase = true },
}

local radiant = {
	staticFlags = {
		stackBase = true,
		stackBaseBuffs = BUFFS,
	},
}

return {
	base = base,
	radiant = radiant,
}
