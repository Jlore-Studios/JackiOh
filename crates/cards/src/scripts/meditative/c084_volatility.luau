--!strict
local J = require("@jackioh")

-- ME-TUNEMULT (R1160): each Buff and Nerf on this is scaled by the running face's multiplier. The
-- multiples are engine constants, kept out of `params` so a Buff can never raise its own.
local base = {
	staticFlags = {
		tuneMultiplier = {
			upgrade = J.config.VOLATILITY_TUNE_MULTIPLIER,
			degrade = J.config.VOLATILITY_TUNE_MULTIPLIER,
		},
	},
}

-- "Buffs are three times as effective": Nerfs are left at their normal size.
local radiant = {
	staticFlags = {
		tuneMultiplier = {
			upgrade = J.config.VOLATILITY_RADIANT_BUFF_MULTIPLIER,
		},
	},
}

return {
	base = base,
	radiant = radiant,
}
