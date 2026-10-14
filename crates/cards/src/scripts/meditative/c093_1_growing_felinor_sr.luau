--!strict
local J = require("@jackioh")

-- The next size up the chain, which this card's Death summons (R1180).
local NEXT = "meditative-093-2"

-- Can't be in Defense Position; Death: a fresh next size up into the leftmost open unit zone (R64),
-- on its Radiant face when this is the Radiant face (R1180). With no zone open it summons nothing.
local function growing(radiant: boolean)
	return {
		staticFlags = { neverDefense = true },
		death = function(ctx: J.EffectContext)
			return {
				J.summon({ defId = NEXT, player = "self", radiant = radiant }),
			}
		end,
	}
end

return {
	base = growing(false),
	radiant = growing(true),
}
