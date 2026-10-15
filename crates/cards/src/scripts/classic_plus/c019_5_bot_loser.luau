--!strict
local J = require("@jackioh")

-- "Whenever this destroys a Unit, it gains +{attackGain} Attack." (`when` is read for traps only, so
-- the test is in `run`.) R42's killer: a destroy with no killer is nobody's kill.
local KILL = {
	id = "bot-loser-kill",
	on = { "destroyed" },
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		if event.type ~= "destroyed" then
			return {}
		end
		local me = ctx:self_()
		if type(me) ~= "table" or event.killerId ~= me.id then
			return {}
		end
		return {
			J.buff({ target = { of = "self" }, attack = ctx:param("attackGain") }),
		}
	end,
}

-- "While Berserk: At the start and end of your turn, this attacks your hero." A card that is nowhere
-- hands back a `null` that is not nil, so the test is for a card.
local function berserk_attack(ctx: J.EffectContext)
	local me = ctx:live_self()
	if type(me) == "table" and me.berserk == true then
		return {
			J.forced_attack_own_hero({ attacker = { of = "self" } }),
		}
	end
	return {}
end

local base = {
	triggers = { KILL },
	startOfTurn = berserk_attack,
	endOfTurn = berserk_attack,
	conditionMet = function(ctx: J.ConditionContext): boolean
		return ctx:zone() == "field" and ctx:self_().berserk == true
	end,
}

-- "This can't go Berserk": no flag, and no Berserk attacks printed (R412).
local radiant = {
	triggers = { KILL },
	staticFlags = { neverBerserk = true },
	-- R195: this face prints no condition, so it never glows; R195's list pins the hook on both faces.
	conditionMet = function(ctx: J.ConditionContext): boolean
		return false
	end,
}

return {
	base = base,
	radiant = radiant,
}
