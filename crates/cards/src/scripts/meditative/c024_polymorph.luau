--!strict
local J = require("@jackioh")

-- §7: the Sheep Token.
local SHEEP_TOKEN = "core-t-sheep"

-- R81: "a Unit", either side.
local targets = {
	{
		kind = "target",
		min = 1,
		max = 1,
		filter = {
			side = "any",
			of = { "unit" },
		},
	},
}

local base = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.transform({ target = { of = "chosen" }, defId = SHEEP_TOKEN }),
		}
	end,
}

-- The Units next to the target, on its side and row (§3.1), are listed first and each becomes a
-- base Sheep, whether or not the target changes; then the target.
local radiant = {
	targets = targets,
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({ cards = "each_cards", each = "each_card" }),
			J.transform({ target = { of = "chosen" }, defId = SHEEP_TOKEN }),
		}
	end,
	each_cards = function(ctx: J.EffectContext): { string }
		local ids: { string } = {}
		for _, card in ipairs(ctx:adjacent_to({ of = "chosen" }, {})) do
			ids[#ids + 1] = card.id
		end
		return ids
	end,
	each_card = function(id: string)
		return { J.transform({ instanceId = id, defId = SHEEP_TOKEN }) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
