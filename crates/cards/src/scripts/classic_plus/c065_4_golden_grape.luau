--!strict
local J = require("@jackioh")

-- §8.7: a card on your side of the field or in your hand.
local function targets()
	return {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "ally", of = { "unit", "backrow", "hand" } },
		},
	}
end

local CHOSEN: J.TargetSpec = { of = "chosen" }

-- "The cards next to it": its row neighbours on the field, or its neighbours by index in your hand.
local function neighbours_of(ctx: J.EffectContext, picked: any): { any }
	if picked.zone.z == "field" then
		return ctx:adjacent_to(CHOSEN, {})
	end
	if picked.zone.z ~= "hand" then
		return {}
	end
	local hand = ctx:zone_cards(picked.owner, "hand")
	local at: number? = nil
	for index, card in ipairs(hand) do
		if card.id == picked.id then
			at = index
			break
		end
	end
	if at == nil then
		return {}
	end
	local neighbours = {}
	if at > 1 then
		neighbours[#neighbours + 1] = hand[at - 1]
	end
	if hand[at + 1] ~= nil then
		neighbours[#neighbours + 1] = hand[at + 1]
	end
	return neighbours
end

local base = {
	targets = targets(),
	cry = function(ctx: J.EffectContext)
		return {
			J.set_radiant({ target = CHOSEN }),
		}
	end,
}

local radiant = {
	targets = targets(),
	cry = function(ctx: J.EffectContext)
		local picked = ctx:instance_of(CHOSEN)
		-- No pick left: the reader's `None` crosses as a null, never a table.
		if type(picked) ~= "table" then
			return {}
		end
		local effects = {
			J.set_radiant({ target = CHOSEN }),
		}
		for _, card in ipairs(neighbours_of(ctx, picked)) do
			effects[#effects + 1] = J.set_radiant({ instanceId = card.id })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = radiant,
}
