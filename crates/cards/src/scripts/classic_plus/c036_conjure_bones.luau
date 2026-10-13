--!strict
local J = require("@jackioh")

local BONE_STORM = "classicplus-036-1"

local function conjure_bones(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local key = if ctx:embiggened() then "paidCount" else "count"
			return {
				J.shuffle_into({
					defId = BONE_STORM,
					count = ctx:param(key),
					radiant = radiant,
				}),
			}
		end,
	}
end

return {
	base = conjure_bones(false),
	radiant = conjure_bones(true),
}
