--!strict
local J = require("@jackioh")

-- R548: a play of this card's controller's that has resolved, other than this card's own (R119).
local function your_other_play(ctx: J.EffectContext, event: J.GameEvent): boolean
	if event.type ~= "cardResolved" then
		return false
	end
	if event.player ~= ctx:controller() then
		return false
	end
	local me = ctx:self_()
	if me ~= nil and me.id == event.instanceId then
		return false
	end
	return true
end

-- While this is in your hand: after you play a Unit, summon this (B5 E26).
local arrive = {
	id = "eu-striker-arrive",
	on = { "cardResolved" },
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		if event.type ~= "cardResolved" or not your_other_play(ctx, event) then
			return {}
		end
		if ctx:def_of(event.defId).type ~= "Unit" then
			return {}
		end
		return { J.summon_this() }
	end,
}

-- On the field: after you play a card, return this to your hand (§6.3 Bounce, R78).
local leave = {
	id = "eu-striker-leave",
	on = { "cardResolved" },
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		if not your_other_play(ctx, event) then
			return {}
		end
		return { J.bounce({ target = { of = "self" } }) }
	end,
}

-- The same script: the Radiant face differs only in what the engine reads off the catalog (its doubled
-- stats and Rush).
return {
	base = {
		handTriggers = { arrive },
		triggers = { leave },
	},
	radiant = "base",
}
