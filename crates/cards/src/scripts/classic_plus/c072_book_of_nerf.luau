--!strict
local J = require("@jackioh")

local function cry(ctx: J.EffectContext)
	return {
		J.degrade({ target = { of = "chosen" }, times = ctx:param("times") }),
	}
end

local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "backrow" } },
		},
	},
	cry = cry,
}

-- A broader scope (R275): also a card in your own hand.
local radiant = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "backrow", "hand" } },
		},
	},
	cry = cry,
}

return {
	base = base,
	radiant = radiant,
}
