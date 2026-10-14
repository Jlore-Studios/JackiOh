--!strict
local J = require("@jackioh")

-- The cost bracket each face Discovers from (§8.1).
local BASE_COST = 2
local RADIANT_COST = 3

-- The step the Discover's answer re-enters (`resume.chosen`).
local CHOSEN = "chosen"

local function discover(cost: number): { J.EffectCall }
	return {
		J.discover_from_catalog({
			step = CHOSEN,
			query = { cost = cost, notTags = { "Token" } },
			count = 3,
		}),
	}
end

-- §10.6: a Discover's answer is the def id it picked, or nothing when the prompt never opened.
local function picked(ctx: J.EffectContext): string?
	return ctx:chosen_options()[1]
end

local base = {
	cry = function(ctx: J.EffectContext)
		return discover(BASE_COST)
	end,
	resume = {
		[CHOSEN] = function(ctx: J.EffectContext)
			local defId = picked(ctx)
			if defId == nil then
				return {}
			end
			return { J.add_to_hand({ defId = defId }) }
		end,
	},
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return discover(RADIANT_COST)
	end,
	resume = {
		[CHOSEN] = function(ctx: J.EffectContext)
			local defId = picked(ctx)
			if defId == nil then
				return {}
			end
			-- A permanent `costMod`, not a `costOverride` (§8.1): R65 starts from the override in place
			-- of the printed cost, which would erase any other discount the card carries. The 1 is the
			-- declared number `discount` (R386).
			return { J.add_to_hand({ defId = defId, costMod = -ctx:param("discount") }) }
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
