--!strict
local J = require("@jackioh")

-- The prompt's Instance picks, in order; see `c011_mind_melt.luau` for why the count is probed.
local function picked_count(ctx: J.EffectContext): number
	local index = 0
	while true do
		local spec: J.TargetSpec = { of = "chosen", index = index }
		if type(ctx:instance_of(spec)) ~= "table" then
			return index
		end
		index += 1
	end
end

-- Your graveyard's Units, oldest first (a graveyard is chronological).
local function graveyard_unit_ids(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "graveyard")) do
		if ctx:def_of(card.defId).type == "Unit" then
			ids[#ids + 1] = card.id
		end
	end
	return ids
end

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.choose_pick({
				step = "picked",
				from = { { zone = "graveyard" } },
				filter = { type = "Unit" },
				max = ctx:zone_count(ctx:controller(), "graveyard"),
				budget = ctx:param("budget"),
				prompt = "Summon Units from your graveyard within the budget",
			}),
			J.exile({ target = { of = "self" } }),
		}
	end,
	resume = {
		picked = function(ctx: J.EffectContext)
			local effects: { J.EffectCall } = {}
			local count = picked_count(ctx)
			for index = 0, count - 1 do
				effects[#effects + 1] =
					J.summon({ instance = { of = "chosen", index = index } })
			end
			return effects
		end,
	},
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({ cards = "each_cards", each = "each_card" }),
			J.exile({ target = { of = "self" } }),
		}
	end,
	each_cards = graveyard_unit_ids,
	each_card = function(id: string)
		return { J.summon({ instance = { of = "instance", instanceId = id } }) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
