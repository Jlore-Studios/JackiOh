--!strict
local J = require("@jackioh")

-- The prompt's Instance picks, in order. There is no `targets` reader, so the count is probed:
-- `instance_of` answers the prompt's picks as they were offered (`chosenFrom`, §10.6) and nil past
-- the end, and the prompt blocks its action between the offer and the answer, so the first nil
-- index is exactly the pick count. The probe is a pure read: the effects below name
-- `{ of = "chosen", index }` exactly as the Rust does, resolving when each effect resolves.
local function picked_count(ctx: J.EffectContext): number
	local index = 0
	while true do
		local spec: J.TargetSpec = { of = "chosen", index = index }
		if type(ctx:instance_of(spec)) ~= "table" then
			return index
		end
		index += 1
	end
end

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.choose_from_hand({
				of = "enemy",
				count = ctx:param("cards"),
				step = "exile",
				prompt = "Look at your opponent's hand: exile a card from it",
			}),
		}
	end,
	resume = {
		exile = function(ctx: J.EffectContext)
			local effects: { J.EffectCall } = {}
			local count = picked_count(ctx)
			for index = 0, count - 1 do
				effects[#effects + 1] = J.exile({ target = { of = "chosen", index = index } })
			end
			return effects
		end,
	},
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.choose_cost_in_hand({
				of = "enemy",
				step = "cost",
				prompt = "Look at your opponent's hand: choose a cost",
			}),
		}
	end,
	resume = {
		cost = function(ctx: J.EffectContext)
			local cost = ctx:chosen_number()
			if cost == nil then
				return {}
			end
			return {
				J.exile_matching({ zones = { "hand" }, player = "enemy", cost = cost }),
			}
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
