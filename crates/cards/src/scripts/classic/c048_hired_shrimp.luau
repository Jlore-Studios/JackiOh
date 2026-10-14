--!strict
local J = require("@jackioh")

local ID = "classic-048"

-- A card's `loc` off the catalog (a fused card's is its ingredients' sum), never a literal.
local function loc_of(ctx: any, def_id: string): number
	return ctx:def_of(def_id).loc or 0
end

local function longer(ctx: any, def_id: string, shrimp_def_id: string): boolean
	return loc_of(ctx, def_id) > loc_of(ctx, shrimp_def_id)
end

-- The Radiant face's pick: a permanent whose card is longer, except a face-down card the chooser may
-- not read, always offered and judged at resolution (R177).
local TARGET_CHECKS = {
	longer = function(args)
		local candidate = args:candidate()
		if candidate == nil then
			return false
		end
		for _, player in ipairs(args:unreadable_by(candidate.id)) do
			if player == args:player() then
				return true
			end
		end
		return longer(args, candidate.defId, args:self_().defId)
	end,
}

-- R397: destroyed only if its `loc` is greater than Hired Shrimp's, else the Cry fizzles.
local function cry(ctx: J.EffectContext)
	local target = ctx:instance_of({ of = "chosen" })
	local me = ctx:self_()
	local shrimp_def_id = if me ~= nil then me.defId else ID
	if target ~= nil and longer(ctx, target.defId, shrimp_def_id) then
		return { J.destroy({ target = { of = "chosen" } }) }
	end
	return {}
end

-- The Radiant face names its target check in the declaration's filter.
local base = {
	targets = {
		{ kind = "target", min = 1, max = 1, filter = { side = "any", of = { "unit", "backrow" } } },
	},
	targetChecks = TARGET_CHECKS,
	cry = cry,
}

local radiant = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "backrow" }, check = "longer" },
		},
	},
	targetChecks = TARGET_CHECKS,
	cry = cry,
}

return {
	base = base,
	radiant = radiant,
}
