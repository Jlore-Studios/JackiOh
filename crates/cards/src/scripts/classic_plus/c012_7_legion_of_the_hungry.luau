--!strict
local J = require("@jackioh")

-- R408's Cry reading: {cards} different random cards of your deck (R60), all of them if fewer, are
-- exiled, then the Units among them are summoned out of exile in the order they were exiled. The
-- picks are drawn as the Cry resolves; nothing in this list can pause between them.
local function legion(ctx: J.EffectContext): { J.EffectCall }
	local library = ctx:zone_cards(ctx:controller(), "library")
	local shuffled = ctx:rng_shuffle(library)
	local count = math.max(ctx:param("cards"), 0)
	local picks = {}
	local units = {}
	for at, card in ipairs(shuffled) do
		if at > count then
			break
		end
		picks[#picks + 1] = card.id
		if ctx:card_type_of(card.id) == "Unit" then
			units[#units + 1] = card.id
		end
	end
	return {
		J.for_each_card({ cards = picks, each = "each_card" }),
		-- A summon that finds no zone leaves the card in exile, and on its old face.
		J.for_each_card({ cards = units, each = "each_card_2" }),
	}
end

local function exile(id: string): { J.EffectCall }
	return { J.exile({ target = { of = "instance", instanceId = id } }) }
end

local base = {
	cry = legion,
	each_card = exile,
	each_card_2 = function(id: string)
		return { J.summon({ instance = { of = "instance", instanceId = id }, radiant = false }) }
	end,
}

local radiant = {
	cry = legion,
	each_card = exile,
	each_card_2 = function(id: string)
		return { J.summon({ instance = { of = "instance", instanceId = id }, radiant = true }) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
