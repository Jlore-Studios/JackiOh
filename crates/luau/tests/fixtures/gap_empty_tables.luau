--!strict
local J = require("@jackioh")

-- Gap 1: the empty-table round trip. The `cards` hook answers `{}` for a
-- list; the cry round-trips the hand (whose instances carry empty lists)
-- through `numbering_order`, and reads `{}` straight back as a list too.
local base = {
	cry = function(ctx: J.EffectContext)
		local me = ctx:controller()
		local hand = ctx:zone_cards(me, "hand")
		local ordered = ctx:numbering_order(hand)
		local empty: { any } = ctx:numbering_order({})
		return {
			J.remember({
				key = "gap1",
				value = { handSize = #hand, orderedSize = #ordered, emptySize = #empty },
			}),
			J.for_each_card({ cards = "each_cards", each = "each_card" }),
		}
	end,
	each_cards = function(_ctx: J.EffectContext)
		return {}
	end,
	each_card = function(_id: string)
		return { J.remember({ key = "gap1_each", value = { ran = true } }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
