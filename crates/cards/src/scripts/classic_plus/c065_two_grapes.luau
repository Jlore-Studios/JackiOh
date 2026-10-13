--!strict
local J = require("@jackioh")

local function two_grapes(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_rolled_grapes({
					count = ctx:param("grapes"),
					radiant = if radiant then true else nil,
				}),
			}
		end,
	}
end

return {
	base = two_grapes(false),
	radiant = two_grapes(true),
}
