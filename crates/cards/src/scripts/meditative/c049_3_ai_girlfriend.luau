--!strict
local J = require("@jackioh")

-- Whether this stands animated in a unit zone: a backrow card's forced attacks find no target.
local function animated(ctx: J.EffectContext): boolean
	local me = ctx:live_self()
	return me ~= nil and me.zone.z == "field" and me.zone.row == "units"
end

-- Start of turn is Moths to the Flame's `forced_attacks_on` self (R53): every enemy Unit attacks in
-- lane order, each its own combat. After each combat this was the target of (ME-AFTER-ATTACKED,
-- MD-C26), the attacker still on the field is Nerfed `times` times. Both faces share the one script:
-- the Radiant face's two Nerfs are its declared `times`.
local base = {
	startOfTurn = function(ctx: J.EffectContext)
		if not animated(ctx) then
			return {}
		end
		local this: J.TargetSpec = { of = "self" }
		return { J.forced_attacks_on({ target = this, attackers = "enemy" }) }
	end,
	afterAttacked = function(ctx: J.EffectContext)
		local facts = ctx:after_attacked_of()
		if facts == nil then
			return {}
		end
		-- MD-C26: Nerf the attacker only if it is still on the field.
		if not facts.attackerSurvived then
			return {}
		end
		return { J.degrade({ instanceId = facts.attackerId, times = ctx:param("times") }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
