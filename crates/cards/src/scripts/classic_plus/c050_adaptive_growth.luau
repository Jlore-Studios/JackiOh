--!strict
local J = require("@jackioh")

-- "If you control fewer Units than your opponent": the tops of their piles, read now.
-- `active_units_of` is bound on every context (#464 gap 3); the condition context's type does not
-- list it, so `conditionMet` passes its context as `any`.
local function fewer(ctx: any): boolean
	local me = ctx:controller()
	return #ctx:active_units_of(me) < #ctx:active_units_of(ctx:opponent_of(me))
end

-- R195: the same `fewer` the Cry branches on, so the glow and the branch agree. One function on
-- both faces.
local function condition_met(ctx: J.ConditionContext): boolean
	return ctx:zone() == "hand" and fewer(ctx)
end

local base = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		local amount = if fewer(ctx) then -ctx:param("debuff") else ctx:param("buff")
		return {
			J.buff_all_units({ side = "both", attack = amount, health = amount }),
		}
	end,
	conditionMet = condition_met,
}

local radiant = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		if fewer(ctx) then
			local amount = -ctx:param("debuff")
			return {
				J.buff_all_units({ side = "enemy", attack = amount, health = amount }),
			}
		end
		local amount = ctx:param("buff")
		return {
			J.buff_all_units({ side = "self", attack = amount, health = amount }),
		}
	end,
	conditionMet = condition_met,
}

return {
	base = base,
	radiant = radiant,
}
