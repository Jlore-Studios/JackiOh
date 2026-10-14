--!strict
local J = require("@jackioh")

-- #90.1 CN-Virus, the token this card shuffles (§7).
local VIRUS = "core-090-1"

-- `radiant_virus` is the whole of the radiant text.
local function injection(radiant_virus: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.shuffle_into({
					defId = VIRUS,
					count = 1,
					player = "enemy",
					radiant = radiant_virus,
				}),
			}
		end,
	}
end

return {
	base = injection(false),
	radiant = injection(true),
}
