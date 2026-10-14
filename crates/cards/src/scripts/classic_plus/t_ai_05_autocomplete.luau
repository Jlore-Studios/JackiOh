--!strict
local J = require("@jackioh")

-- §8.7 Radiant: "It costs (0)."
local RADIANT_COST = 0

local function autocomplete(ctx: J.EffectContext, cost_override: number?)
	local last = ctx:last_face_up_played(ctx:opponent_of(ctx:controller()))
	-- Nothing played yet: the reader's `None` crosses as a null, never a table.
	if type(last) ~= "table" then
		return {}
	end
	local args: J.AddToHandArgs = { defId = last.defId, radiant = last.radiant }
	-- R1300: a copy of a Chinese card is Chinese.
	if last.chinese == true then
		args.chinese = true
	end
	if cost_override ~= nil then
		args.costOverride = cost_override
	end
	return {
		J.add_to_hand(args),
	}
end

local base = {
	cry = function(ctx: J.EffectContext)
		return autocomplete(ctx, nil)
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return autocomplete(ctx, RADIANT_COST)
	end,
}

return {
	base = base,
	radiant = radiant,
}
