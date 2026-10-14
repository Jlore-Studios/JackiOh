--!strict
local J = require("@jackioh")

-- Patch v0.3.9 (#831): "Cry: Buff this", one Buff on itself, which its own multiplier scales.
local function buff_this(ctx: J.EffectContext)
	return { J.upgrade({ target = { of = "self" } }) }
end

-- ME-TUNEMULT (R1160): each Buff and Nerf on this is scaled by the running face's multiplier. The
-- multiples are engine constants, kept out of `params` so a Buff can never raise its own.
local base = {
	staticFlags = {
		tuneMultiplier = {
			upgrade = J.config.VOLATILITY_TUNE_MULTIPLIER,
			degrade = J.config.VOLATILITY_TUNE_MULTIPLIER,
		},
	},
	cry = buff_this,
}

-- "Buffs are three times as effective": Nerfs are left at their normal size.
local radiant = {
	staticFlags = {
		tuneMultiplier = {
			upgrade = J.config.VOLATILITY_RADIANT_BUFF_MULTIPLIER,
		},
	},
	cry = buff_this,
}

return {
	base = base,
	radiant = radiant,
}
