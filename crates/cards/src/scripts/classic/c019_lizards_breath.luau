--!strict
local J = require("@jackioh")

-- How many of the largest piles count: "Your largest pile", radiant "Your two largest piles".
local BASE_PILES_COUNTED = 1
local RADIANT_PILES_COUNTED = 2

-- The three piles in the text's order, which is the tie order: each pile's zone and its name in the text.
local PILES: { { zone: J.OffFieldZone, name: string } } = {
	{ zone = "library", name = "Deck" },
	{ zone = "graveyard", name = "Graveyard" },
	{ zone = "exile", name = "Exile" },
}

type RankedPile = { name: string, size: number }

-- The `count` largest of a player's piles, largest first, a tie going to the pile listed first.
-- `size_of` counts one pile: the effect and the condition contexts each read it their own way.
local function counting_piles(size_of: (zone: J.OffFieldZone) -> number, count: number): { RankedPile }
	local sized: { RankedPile } = {}
	for _, pile in ipairs(PILES) do
		sized[#sized + 1] = { name = pile.name, size = size_of(pile.zone) }
	end
	-- Stable (SURFACE §4.4.1): the larger first, a tie by the text's order.
	local ranked = J.sort_by(sized, function(pile: RankedPile)
		return -pile.size
	end)
	local out: { RankedPile } = {}
	for i = 1, math.min(count, #ranked) do
		out[#out + 1] = ranked[i]
	end
	return out
end

local function counts(piles: { RankedPile }, name: string): boolean
	for _, pile in ipairs(piles) do
		if pile.name == name then
			return true
		end
	end
	return false
end

local function lizards_breath(piles_counted: number, label: string)
	return {
		-- §8 Conventions: any unit or hero, either side.
		targets = {
			{
				kind = "target",
				min = 1,
				max = 1,
				filter = {
					side = "any",
					of = { "unit", "hero" },
				},
			},
		},
		cry = function(ctx: J.EffectContext)
			local me = ctx:controller()
			local counting = counting_piles(function(zone: J.OffFieldZone): number
				return ctx:zone_count(me, zone)
			end, piles_counted)
			local extra = if counts(counting, "Exile") then ctx:param("extraDamage") else 0
			local effects = {
				J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") + extra }),
			}
			if counts(counting, "Deck") then
				effects[#effects + 1] = J.draw({ count = ctx:param("draw") })
			end
			if counts(counting, "Graveyard") then
				effects[#effects + 1] = J.gain_mana({ amount = ctx:param("mana") })
			end
			return effects
		end,
		-- R280: the pile or piles that would count now, in rank order.
		preview = function(ctx: J.ConditionContext)
			local me = ctx:controller()
			local counting = counting_piles(function(zone: J.OffFieldZone): number
				return ctx:zone_count(me, zone)
			end, piles_counted)
			local out = {}
			for _, pile in ipairs(counting) do
				out[#out + 1] = {
					label = label,
					value = pile.size,
					display = pile.name,
				}
			end
			return out
		end,
	}
end

return {
	base = lizards_breath(BASE_PILES_COUNTED, "Your largest pile"),
	radiant = lizards_breath(RADIANT_PILES_COUNTED, "Your two largest piles"),
}
