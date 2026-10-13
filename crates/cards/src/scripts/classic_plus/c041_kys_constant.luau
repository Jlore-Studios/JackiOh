--!strict
local J = require("@jackioh")

-- "change … to 3".
local CONSTANT = 3
-- §6.3 Discover: up to 3 options.
local OFFERED = 3

-- R81: a declared hand pick among your other hand cards with a number (R386) not already 3.
local TARGETS = {
	{
		kind = "hand",
		min = 1,
		max = 1,
		filter = { of = { "hand" }, excludeSelf = true, check = "number" },
	},
}

local function target_checks()
	return {
		number = function(args)
			local candidate = args:candidate()
			if candidate == nil then
				return false
			end
			for _, entry in ipairs(args:numbers_on(candidate.id)) do
				if entry.value ~= CONSTANT then
					return true
				end
			end
			return false
		end,
	}
end

-- Base: one of its numbers at random (R60).
local base = {
	targets = TARGETS,
	targetChecks = target_checks(),
	cry = function(ctx: J.EffectContext)
		return { J.set_number({ target = { of = "chosen" }, which = "random", value = CONSTANT }) }
	end,
}

-- Radiant: a Discover of up to 3 of them; the number picked off it becomes 3.
local radiant = {
	targets = TARGETS,
	targetChecks = target_checks(),
	cry = function(ctx: J.EffectContext)
		return {
			J.discover_number({
				target = { of = "chosen" },
				value = CONSTANT,
				count = OFFERED,
				step = "picked",
			}),
		}
	end,
	resume = {
		picked = function(ctx: J.EffectContext)
			-- `ChosenTuningNumber` crosses with its Rust field names (`instance_id`, `which`).
			local pick = ctx:chosen_tuning_number()
			if pick == nil then
				return {}
			end
			return { J.set_number({ instanceId = pick.instance_id, which = pick.which, value = CONSTANT }) }
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
