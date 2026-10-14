--!strict
local J = require("@jackioh")

-- §7: the Sheep Token.
local SHEEP_TOKEN = "core-t-sheep"

-- The Units adjacent to this (§3.1: lanes N − 1 and N + 1 on its own side and row, the tops of piles,
-- R13), listed once before the first transform.
local function neighbours(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	for _, card in ipairs(ctx:adjacent_to({ of = "self" }, {})) do
		ids[#ids + 1] = card.id
	end
	return ids
end

-- Each neighbour becomes a Sheep Token in place (R691), a Radiant one on the Radiant face.
local function golly_bob_howdy(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.for_each_card({ cards = "each_cards", each = "each_card" }),
			}
		end,
		each_cards = neighbours,
		each_card = function(id: string)
			return { J.transform({ instanceId = id, defId = SHEEP_TOKEN, radiant = radiant }) }
		end,
	}
end

return {
	base = golly_bob_howdy(false),
	radiant = golly_bob_howdy(true),
}
