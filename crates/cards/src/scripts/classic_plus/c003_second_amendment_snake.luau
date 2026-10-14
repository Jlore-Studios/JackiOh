--!strict
local J = require("@jackioh")

-- R280: the words the preview's value follows, on both faces.
local HITS_LABEL = "for each Plague Counter on this"

-- One hit per Plague Counter on the card: the Death's count and the preview's (R78: last-known).
-- A card that is nowhere hands back a `null` that is not nil, so the test is for a card.
local function hits(card: any): number
	if type(card) ~= "table" then
		return 0
	end
	local plague = card.counters.plague
	if type(plague) == "number" then
		return plague
	end
	return 0
end

-- The Radiant face differs only in its stats and its `tokens` value (3), both catalog data.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		return {
			J.place_plague({ amount = ctx:param("tokens") }),
		}
	end,
	death = function(ctx: J.EffectContext)
		local per_hit = ctx:param("damage")
		return {
			J.damage_split({
				amount = hits(ctx:self_()) * per_hit,
				perHit = per_hit,
				among = "enemies",
			}),
		}
	end,
	preview = function(ctx: J.ConditionContext)
		return {
			{ label = HITS_LABEL, value = hits(ctx:self_()) },
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
