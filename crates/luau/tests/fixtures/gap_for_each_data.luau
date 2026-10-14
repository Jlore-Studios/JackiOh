--!strict
local J = require("@jackioh")

-- Gap 2: `for_each_card` with `data`, passed to its named `cards` and `each`
-- functions the way `after_state_check` passes it. The amount the `each`
-- hook deals comes from `data`, and the `cards` hook only answers when
-- `data` says so.
local base = {
	cry = function(_ctx: J.EffectContext)
		return {
			J.for_each_card({ cards = "each_cards", each = "each_card", data = { amount = 2, go = true } }),
		}
	end,
	each_cards = function(ctx: J.EffectContext, data: any)
		if data.go ~= true then
			return {}
		end
		local ids = {}
		for _, unit in ipairs(ctx:active_units_of(ctx:opponent_of(ctx:controller()))) do
			ids[#ids + 1] = unit.id
		end
		return ids
	end,
	each_card = function(id: string, data: any)
		return { J.damage({ to = { of = "instance", instanceId = id }, amount = data.amount }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
