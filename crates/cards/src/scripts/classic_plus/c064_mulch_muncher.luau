--!strict
local J = require("@jackioh")

local ID = "classicplus-064"

-- Core #100's pattern (R55): the printed (10) (`J.cost`, TS `queryCost(def)`, never restated) less
-- the declared `discount` per Fruit-tagged card its player has played this game (casts count,
-- R70), floored at 0. R584: the discount is a price for a play, so it holds where a play takes the
-- card from — its player's hand, or a graveyard a permission lets them play it from (E11);
-- anywhere else it costs its printed (10) (R65). The graveyard test reads the card by id, as
-- `in_own_graveyard` does: a probe's copy of the card is the same card there.
local base = {
	cost = function(ctx: J.CostArgs): number
		local printed = J.cost(ID)
		local card = ctx:instance()
		local at = card.zone
		local for_play = at.z == "hand" or (at.z == "graveyard" and ctx:playable_from_graveyard(card.id))
		if not for_play then
			return printed
		end
		local discount = ctx:param("discount")
		return math.max(printed - discount * ctx:played_this_game_with_tag(at.player, "Fruit"), 0)
	end,
}

-- The keywords are the faces' printed ones, so both faces run this one script.
return {
	base = base,
	radiant = "base",
}
