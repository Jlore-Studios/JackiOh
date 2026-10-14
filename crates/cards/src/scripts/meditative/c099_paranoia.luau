--!strict
local J = require("@jackioh")

-- "Cry: Draw 1": the declared number `draw` (R386), on both faces.
local function cry(ctx: J.EffectContext)
	return {
		J.draw({ count = ctx:param("draw") }),
	}
end

-- "Aura: Your Spells have Cloaked." (R1044): Cloaked for the Spells in its controller's hand while
-- it acts (the engine's `cloak_hand_aura`).
local function cloak_spells(ctx: J.HookArgs)
	return {
		J.cloak_hand_aura({ type = "Spell" }),
	}
end

-- The Radiant face's "Spells you reveal have Echo +{echo}" (R1045), read as the Spell reveals.
return {
	base = {
		cry = cry,
		aura = cloak_spells,
	},
	radiant = {
		cry = cry,
		aura = cloak_spells,
		revealEcho = function(ctx: J.HookArgs)
			return ctx:param("echo")
		end,
	},
}
