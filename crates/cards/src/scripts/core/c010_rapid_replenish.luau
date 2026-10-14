--!strict
local J = require("@jackioh")

-- "Combo 3" (§6.2): three or more cards played earlier this turn — the declared number `combo`, a
-- threshold, less being better (R386) — and the draw, the declared number `draw`: 3, 6 on the
-- Radiant face. The only difference between the two faces is how many cards the met Combo draws.
local base = {
	cry = function(ctx: J.EffectContext)
		local combo = ctx:param("combo")
		local count = ctx:param("draw")
		-- The plays before this one, at play time: not this spell, and not a card its step 5 cast.
		local earlier = ctx:played_earlier(ctx:controller(), ctx:live_self())
		if earlier >= combo then
			return {
				J.draw({ count = count }),
			}
		end
		return {}
	end,
	-- R195: hand only. The condition is about a play; a Spell never sits on the field. In hand,
	-- `played_earlier` is every play this turn.
	conditionMet = function(ctx: J.ConditionContext)
		return ctx:zone() == "hand" and ctx:played_earlier(ctx:controller(), ctx:self_()) >= ctx:param("combo")
	end,
}

return {
	base = base,
	radiant = "base",
}
