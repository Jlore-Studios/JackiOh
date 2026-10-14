--!strict
local J = require("@jackioh")

-- The printed "a Plague Counter": one per start of turn.
local PLAGUE_PER_TURN = 1

-- The tokens on itself as it died (R89), never below 0, or none when it has no self. A card that is
-- nowhere hands back a `null` that is not nil, so the test is for a card.
local function tokens_on_self(card: any): number
	if type(card) ~= "table" then
		return 0
	end
	local plague = card.counters.plague
	if type(plague) ~= "number" then
		return 0
	end
	return math.max(plague, 0)
end

-- One face: whether the Books it adds are Radiant.
local function book_worm(radiant: boolean)
	return {
		startOfTurn = function(ctx: J.EffectContext)
			local me: J.TargetSpec = { of = "self" }
			return {
				J.place_plague({ target = me, amount = PLAGUE_PER_TURN }),
			}
		end,
		death = function(ctx: J.EffectContext)
			local args: J.AddRandomFromCatalogArgs = {
				query = { tags = { "Book" } },
				count = tokens_on_self(ctx:self_()),
			}
			if radiant then
				args.radiant = true
			end
			return {
				J.add_random_from_catalog(args),
			}
		end,
		-- R280: the tokens stacked on itself are the Books its Death would add, the public count.
		preview = function(ctx: J.ConditionContext)
			return {
				{ label = "Plague Counter", value = tokens_on_self(ctx:self_()) },
			}
		end,
	}
end

return {
	base = book_worm(false),
	radiant = book_worm(true),
}
