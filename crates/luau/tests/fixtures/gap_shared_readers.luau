--!strict
local J = require("@jackioh")

-- Gaps 3 and 4: one reader table on every context. The condition, the
-- preview and the cost hook read the turn, the plays before this one, the
-- game counters and the board, the way an effect hook does.
local base = {
	conditionMet = function(ctx: J.ConditionContext)
		local me = ctx:controller()
		local turn = ctx:turn()
		return turn.number >= 1
			and turn.active == me
			and ctx:played_earlier(me, ctx:self_()) >= 0
			and ctx:params_view() == nil
	end,
	preview = function(ctx: J.ConditionContext)
		local me = ctx:controller()
		return {
			{ label = "gap", value = #ctx:open_zones(me, "units") + #ctx:fusable_permanents_of(me) },
		}
	end,
	cost = function(ctx: J.CostArgs)
		return ctx:turn().number
	end,
	cry = function(_ctx: J.EffectContext)
		return {}
	end,
}

return {
	base = base,
	radiant = "base",
}
