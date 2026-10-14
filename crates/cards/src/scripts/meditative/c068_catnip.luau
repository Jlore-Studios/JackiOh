--!strict
local J = require("@jackioh")

-- The engine's `card_type_of_face` for the face that resolved: that face's own `type`, else the
-- definition's. The face is the event's; `ctx:card_type_of_face` would read this card's own.
local function face_type(ctx: J.EffectContext, defId: string, radiant: boolean): J.CardType
	local def = ctx:def_of(defId)
	local face = if radiant then def.radiant else def.base
	return face.type or def.type
end

-- An Animated Field Trap (R383), as C #5 Tesla: fires once a Spell the opponent played or cast has
-- resolved (R17, MD-D25, R1063), then animates; already a Unit, later Spells do nothing.
local reveal = {
	id = "catnip-reveal",
	on = { "cardResolved" },
	when = function(ctx: J.EffectContext, event: J.GameEvent): boolean
		if event.type ~= "cardResolved" then
			return false
		end
		if event.player == ctx:controller() then
			return false
		end
		if face_type(ctx, event.defId, event.radiant == true) ~= "Spell" then
			return false
		end
		local me = ctx:live_self()
		return not (me ~= nil and ctx:is_animated(me.id))
	end,
	run = function(_ctx: J.EffectContext, _event: J.GameEvent)
		return { J.animate({}) }
	end,
}

-- The Radiant face differs only in stats and Taunt, both catalog data.
return {
	base = {
		triggers = { reveal },
	},
	radiant = "base",
}
