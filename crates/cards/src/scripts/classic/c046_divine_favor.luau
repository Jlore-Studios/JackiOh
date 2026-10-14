--!strict
local J = require("@jackioh")

-- R280: the label the preview's value follows, an exact substring of both faces' text.
local PREVIEW_LABEL = "Draw"

-- The other player. A `ConditionContext` binds no `opponent_of`, so the preview names the other seat
-- itself; the Cry asks `ctx:opponent_of`.
local function other(player: J.PlayerId): J.PlayerId
	if player == "p1" then
		return "p2"
	end
	return "p1"
end

-- The cards still wanted: `multiplier ×` the opponent's hand less your own, never below 0. `leaving`
-- counts the cards about to leave your hand before the draws begin (this card, asked in hand).
local function draws_wanted(yours: number, theirs: number, multiplier: number, leaving: number): number
	return math.max(multiplier * theirs - (yours - leaving), 0)
end

-- The same script on both faces: the Radiant face differs only in its declared multiplier (2), read
-- through `param`.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.draw_while({ hook = "draw_more" }),
		}
	end,
	-- Before each draw: whether the hand is still short of the mark.
	draw_more = function(ctx: J.EffectContext): boolean
		local me = ctx:controller()
		local yours = ctx:zone_count(me, "hand")
		local theirs = ctx:zone_count(ctx:opponent_of(me), "hand")
		return draws_wanted(yours, theirs, ctx:param("multiplier"), 0) > 0
	end,
	preview = function(ctx: J.ConditionContext)
		local me = ctx:controller()
		local self_ = ctx:self_()
		local in_hand = 0
		if ctx:zone() == "hand" and self_.zone.z == "hand" then
			in_hand = 1
		end
		local yours = ctx:zone_count(me, "hand")
		local theirs = ctx:zone_count(other(me), "hand")
		return {
			{ label = PREVIEW_LABEL, value = draws_wanted(yours, theirs, ctx:param("multiplier"), in_hand) },
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
