--!strict
local J = require("@jackioh")

-- C #16, the Book of Flame this card names (its `refs`).
local BOOK_OF_FLAME = "classic-016"

-- "Discard {discards} cards": that many at random, or the whole hand when it holds no more.
local function discard_cards(ctx: J.EffectContext): { J.EffectCall }
	local count = ctx:param("discards")
	local held = ctx:zone_count(ctx:controller(), "hand")
	if held == 0 then
		return {}
	end
	if held <= count then
		return { J.discard_hand({ player = "self" }) }
	end
	return { J.discard_random({ count = count }) }
end

-- Activate (R384), once per turn: a this-turn player modifier (`replacePlays`, R449) that replaces
-- each card its controller plays by a new Book of Flame of the face's kind.
local function pact(ctx: J.EffectContext, radiant: boolean): { J.EffectCall }
	return {
		J.add_player_modifier({
			mod = {
				kind = "replacePlays",
				defId = BOOK_OF_FLAME,
				radiant = radiant,
				expiry = { ["until"] = "thisTurn", turn = ctx:turn().number },
			},
		}),
	}
end

local base = {
	cry = discard_cards,
	activations = {
		{
			id = "pact",
			label = "This turn, each card you play is replaced by a Book of Flame",
			uses = 1,
			run = function(ctx: J.EffectContext)
				return pact(ctx, false)
			end,
		},
	},
}

local radiant = {
	cry = discard_cards,
	activations = {
		{
			id = "pact",
			label = "This turn, each card you play is replaced by a Radiant Book of Flame",
			uses = 1,
			run = function(ctx: J.EffectContext)
				return pact(ctx, true)
			end,
		},
	},
}

return {
	base = base,
	radiant = radiant,
}
