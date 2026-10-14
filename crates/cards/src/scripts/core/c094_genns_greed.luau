--!strict
local J = require("@jackioh")

-- §8 #94: the one cost the draw clause names.
local DRAWN_COST = 2

-- R66: every card in your library that costs 2 RIGHT NOW, X-cost cards exempt, in library order
-- (top down), which is the order they are drawn in. By instance id, which is what
-- `for_each_card` keeps.
local function drawn_cards(ctx: J.EffectContext): { string }
	local ids: { string } = {}
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "library")) do
		if not ctx:is_x_cost(card.id) and ctx:effective_cost(card.id) == DRAWN_COST then
			ids[#ids + 1] = card.id
		end
	end
	return ids
end

-- §8: base "gain 2 mana"; radiant "Gain 6" — the declared number `mana` (R386).
local function greed()
	return {
		cry = function(ctx: J.EffectContext)
			local mana = ctx:param("mana")
			return {
				-- §8 "Draw every 2-cost card from your library": one draw per card, so each counts on
				-- the draw counter, emits its own `drawn` event and meets the hand cap on its own
				-- (§2.4, R4, R55). The set is read once, as the clause begins (R66), so a drawn card
				-- that is cast and asks does not reshape the draws still owed after the answer (R113).
				-- The ids are handed over outright rather than through an `each_cards` function: a
				-- cards function's empty answer crosses as `{}`, which the host refuses, and this
				-- clause is the Cry's first effect, so nothing moves between this read and the
				-- clause's start; the clause keeps the ids as it begins, as it keeps a function's.
				J.for_each_card({ cards = drawn_cards(ctx), each = "each_card" }),
				-- §8 "exile every odd-cost card in your library, hand and GY (X-cost cards exempt)".
				-- R135's order is `exile_matching`'s default — library, then hand, then graveyard —
				-- and it runs after the draws, so a card this play just drew is in hand, even, and
				-- spared.
				J.exile_matching({ parity = "odd", exemptXCost = true }),
				-- §2.3: temporary mana, which may take current above max.
				J.gain_mana({ amount = mana }),
			}
		end,
		each_card = function(id: string)
			return { J.draw_from_library({ instanceId = id }) }
		end,
	}
end

return {
	base = greed(),
	radiant = greed(),
}
