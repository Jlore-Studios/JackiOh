--!strict
local J = require("@jackioh")

-- §7, §8.7: the token the Cry shuffles in (TS `cardDef("classicplus-075-1").id`).
local PACK = "classicplus-075-1"

local function explorer(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.shuffle_into({
					defId = PACK,
					count = ctx:param("packs"),
					radiant = if radiant then true else nil,
				}),
			}
		end,
	}
end

return {
	base = explorer(false),
	radiant = explorer(true),
}
