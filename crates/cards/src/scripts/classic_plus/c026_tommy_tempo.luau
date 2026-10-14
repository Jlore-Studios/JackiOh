--!strict
local J = require("@jackioh")

-- Whether this Cry runs as the cast on draw (R70), not as a play from the hand. A card that is
-- nowhere hands back a `null` that is not nil, so the test is for a card.
local function cast_on_draw(ctx: J.EffectContext): boolean
	local me = ctx:self_()
	return type(me) == "table" and ctx:is_cast_on_draw(me.id)
end

local base = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		if cast_on_draw(ctx) then
			return {
				J.end_turn({}),
			}
		end
		return {}
	end,
}

-- R415: one more main-phase action before the turn ends, the declared number `actions`.
local radiant = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		if cast_on_draw(ctx) then
			return {
				J.end_turn_after_actions({ actions = ctx:param("actions") }),
			}
		end
		return {}
	end,
}

return {
	base = base,
	radiant = radiant,
}
