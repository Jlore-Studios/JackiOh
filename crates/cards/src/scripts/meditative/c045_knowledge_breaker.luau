--!strict
local J = require("@jackioh")

-- M #45.1, the Prime its Death shuffles in.
local PRIME = "meditative-045-1"

-- ME-ALTPLAY: "You may play your Units face-down as Animated Field Traps that reveal at the start
-- of your next turn".
local function face_down_units(ctx: J.HookArgs)
	return {
		{ units = true },
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
		faceDownPlay = face_down_units,
	}
end

return {
	base = breaker(false),
	radiant = breaker(true),
}
