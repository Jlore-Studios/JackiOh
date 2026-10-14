--!strict
local J = require("@jackioh")

-- The Rock this adds each turn (M #39.1).
local ROCK = "meditative-039-1"

-- A Field Spell's start of turn (R62): the Rock, Radiant on the Radiant face.
local function well(radiant: boolean)
	return {
		startOfTurn = function(_ctx: J.EffectContext)
			return { J.add_to_hand({ defId = ROCK, radiant = radiant }) }
		end,
	}
end

return {
	base = well(false),
	radiant = well(true),
}
