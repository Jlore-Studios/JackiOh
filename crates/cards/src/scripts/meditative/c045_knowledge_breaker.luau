--!strict
local J = require("@jackioh")

-- M #45.1, the Prime its Death shuffles in.
local PRIME = "meditative-045-1"

-- "Aura: Your Units have Cloaked." (R1040): Cloaked for the Units in its controller's hand while
-- it acts (the engine's `cloak_hand_aura`).
local function cloak_units(ctx: J.HookArgs)
	return {
		J.cloak_hand_aura({ type = "Unit" }),
	}
end

-- "Nerf every other CN or KY card on the field, in each hand and in each deck" (Radiant: `times`
-- times). "Death: Shuffle a (Radiant) Knowledge Breaker Prime into your deck" (C+ #38's shape).
local function breaker(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local times = ctx:param("times")
			return {
				J.degrade({
					scope = {
						side = "any",
						zones = { "field", "hand", "library" },
						tags = { "CN", "KY" },
						excludeSelf = true,
					},
					times = times,
				}),
			}
		end,
		death = function(ctx: J.EffectContext)
			return {
				J.shuffle_into({
					defId = PRIME,
					count = 1,
					radiant = radiant,
				}),
			}
		end,
		aura = cloak_units,
	}
end

return {
	base = breaker(false),
	radiant = breaker(true),
}
