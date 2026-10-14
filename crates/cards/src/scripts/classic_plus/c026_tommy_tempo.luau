--!strict
local J = require("@jackioh")

-- Whether this Cry runs as the cast on draw (R70), not as a play from the hand. A card that is
-- nowhere hands back a `null` that is not nil, so the test is for a card.
local function cast_on_draw(ctx: J.EffectContext): boolean
	local me = ctx:self_()
	return type(me) == "table" and ctx:is_cast_on_draw(me.id)
end

-- Patch v0.3.9 (#831): "Lose all your mana. Gain +{per}/+{per} for each mana lost", the mana left
-- once the play's price is paid, on a cast on draw too (R831010).
local function tempo(ctx: J.EffectContext): { J.EffectCall }
	local lost = ctx:unspent_mana_of(ctx:controller())
	if lost <= 0 then
		return {}
	end
	local gain = lost * ctx:param("per")
	return {
		J.gain_mana({ amount = -lost }),
		J.buff({ target = { of = "self" }, attack = gain, health = gain }),
	}
end

local base = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		local effects = tempo(ctx)
		if cast_on_draw(ctx) then
			effects[#effects + 1] = J.end_turn({})
		end
		return effects
	end,
}

-- R415: one more main-phase action before the turn ends, the declared number `actions`.
local radiant = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		local effects = tempo(ctx)
		if cast_on_draw(ctx) then
			effects[#effects + 1] = J.end_turn_after_actions({ actions = ctx:param("actions") })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = radiant,
}
