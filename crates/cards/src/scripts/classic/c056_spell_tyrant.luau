--!strict
local J = require("@jackioh")

-- R453: `cast_each` with "then exile them" (`afterward = "exile"`), each cast's choices its
-- caster's (R70).
local function cast_each_then_exile(ids: { string }): J.EffectCall
	return J.cast_each({ cards = ids, afterward = "exile" })
end

-- The graveyard's Spells, oldest first. Read when the list reaches it, so a Spell a cast puts in
-- the graveyard is not cast.
local function graveyard_spell_ids(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "graveyard")) do
		if ctx:card_type_of(card.id) == "Spell" then
			ids[#ids + 1] = card.id
		end
	end
	return ids
end

-- The prompt's Instance picks, in order; see `c011_mind_melt.luau` for why they are probed. The
-- ids travel into one `cast_each`, exactly the Rust's picked list.
local function picked_ids(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	local index = 0
	while true do
		local spec: J.TargetSpec = { of = "chosen", index = index }
		local card = ctx:instance_of(spec)
		if type(card) ~= "table" then
			return ids
		end
		ids[#ids + 1] = card.id
		index += 1
	end
end

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.choose_pick({
				step = "cast",
				from = { { zone = "graveyard" } },
				filter = { type = "Spell" },
				max = ctx:param("spells"),
				prompt = "Choose Spells in your graveyard to cast",
			}),
		}
	end,
	resume = {
		cast = function(ctx: J.EffectContext)
			return { cast_each_then_exile(picked_ids(ctx)) }
		end,
	},
}

local radiant = {
	-- The Spells there as the Cry begins, oldest first, read once into the one `cast_each`.
	cry = function(ctx: J.EffectContext)
		return {
			cast_each_then_exile(graveyard_spell_ids(ctx)),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
