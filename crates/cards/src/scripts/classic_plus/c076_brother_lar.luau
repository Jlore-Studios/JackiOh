--!strict
local J = require("@jackioh")

-- §7, §8.7: the token Brother Lar leaves behind, C+ #76.1 Brother Ping.
local BROTHER_PING = "classicplus-076-1"

local base = {
	death = function(ctx: J.EffectContext)
		return {
			J.summon({ defId = BROTHER_PING }),
		}
	end,
}

local radiant = {
	death = function(ctx: J.EffectContext)
		return {
			J.summon({ defId = BROTHER_PING, radiant = true }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
