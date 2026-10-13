--!strict
local J = require("@jackioh")

-- R656: Reborn helps, so a random cast that targets enemies aims this at friends.
local function give_reborn(): J.EffectCall
	local reborn: J.Keyword = { kind = "Reborn" }
	return J.grant_keyword({ target = { of = "chosen" }, keyword = reborn })
end

local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit" } },
			aim = "help",
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			give_reborn(),
		}
	end,
}

local radiant = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit" } },
			aim = "help",
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			give_reborn(),
			J.set_radiant({}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
