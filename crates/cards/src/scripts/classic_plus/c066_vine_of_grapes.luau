--!strict
local J = require("@jackioh")

local function vine_of_grapes(radiant: boolean)
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
	base = vine_of_grapes(false),
	radiant = vine_of_grapes(true),
}
