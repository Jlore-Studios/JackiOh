--!strict
local J = require("@jackioh")

-- C+ #59 All Purpose Apple, named, so no pool.
local APPLE = "classicplus-059"

-- "Activate: Add {apples} All Purpose Apple(s) to your hand", the Apples of the face's kind.
local function order(radiant: boolean)
	return {
		id = "order",
		label = "Add an All Purpose Apple to your hand",
		uses = 1,
		run = function(ctx: J.EffectContext)
			local effects = {}
			for _ = 1, ctx:param("apples") do
				effects[#effects + 1] = J.add_to_hand({ defId = APPLE, radiant = radiant })
			end
			return effects
		end,
	}
end

return {
	base = { activations = { order(false) } },
	radiant = { activations = { order(true) } },
}
