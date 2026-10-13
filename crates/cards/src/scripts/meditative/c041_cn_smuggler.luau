--!strict
local J = require("@jackioh")

-- The Rock this adds first (M #39.1).
local ROCK = "meditative-039-1"

-- Start of turn: the Rock, then a random non-token CN card of every set (R380, R382; never this card,
-- R387, which `add_random_from_catalog` already excludes), both Radiant on the Radiant face. The hand
-- cap burns extras, in that order.
local function smuggler(radiant: boolean)
	return {
		startOfTurn = function(_ctx: J.EffectContext)
			return {
				J.add_to_hand({ defId = ROCK, radiant = radiant }),
				J.add_random_from_catalog({ query = { tags = { "CN" } }, radiant = radiant }),
			}
		end,
	}
end

return {
	base = smuggler(false),
	radiant = smuggler(true),
}
