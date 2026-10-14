--!strict
local J = require("@jackioh")

-- "Whenever THIS takes damage": the event carries the target, so a hit this card dealt (its
-- strike-back, its Trample overflow) is not a hit it took.
local function is_hit_on_self(ctx: J.EffectContext, event: J.GameEvent): boolean
	local me = ctx:self_()
	if me == nil then
		return false
	end
	if event.type ~= "damage" then
		return false
	end
	return event.targetId == me.id
end

-- One random discard per damage instance (R59, R682) from the hand of whoever controls this as the
-- hit lands (R212); an empty hand draws no random number (R129).
local discard = {
	id = "rampaging-rhino-discard",
	on = { "damage" },
	when = is_hit_on_self,
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		if is_hit_on_self(ctx, event) then
			return { J.discard_random({ count = ctx:param("discards"), player = "self" }) }
		end
		return {}
	end,
}

-- Trample is catalog data, so the Radiant face's script is the base's.
return {
	base = {
		triggers = { discard },
	},
	radiant = "base",
}
