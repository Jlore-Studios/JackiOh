--!strict
local J = require("@jackioh")

local VITAL_HEALTH = 13

local function targets(): { J.TargetDecl }
	return {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "hero" } },
		},
	}
end

local function setVitalHealth(): J.EffectCall
	return J.set_health({ to = { of = "chosen" }, value = VITAL_HEALTH })
end

-- Patch v0.3.9 (#831): the base face ends the turn once the health is set.
local base = {
	targets = targets(),
	cry = function(ctx: J.EffectContext)
		return { setVitalHealth(), J.end_turn({}) }
	end,
}

local radiant = {
	targets = targets(),
	cry = function(ctx: J.EffectContext)
		return { setVitalHealth() }
	end,
}

return {
	base = base,
	radiant = radiant,
}
