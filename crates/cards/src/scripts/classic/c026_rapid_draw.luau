--!strict
local J = require("@jackioh")

-- Draw {draw}, then discard {discard}: the draws are §2.4's pipeline, then
-- "then discard" (R682) is random and reads the hand as the list reaches it.
-- With no more cards in hand than the discard asks for, the `cards` hook
-- answers the whole hand in hand order; with more it answers `{}`, and the
-- random discard takes exactly that many. Both faces run this: the Radiant
-- face's 5 draws are its declared `draw`, which `param` reads off the
-- running face.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.draw({ count = ctx:param("draw") }),
			J.for_each_card({ cards = "whole_hand", each = "each_discard" }),
			J.discard_random({ count = ctx:param("discard") }),
		}
	end,
	whole_hand = function(ctx: J.EffectContext)
		local me = ctx:controller()
		if ctx:zone_count(me, "hand") > ctx:param("discard") then
			return {}
		end
		local ids: { string } = {}
		for _, card in ipairs(ctx:zone_cards(me, "hand")) do
			ids[#ids + 1] = card.id
		end
		return ids
	end,
	each_discard = function(id: string)
		return { J.discard({ target = { of = "instance", instanceId = id } }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
