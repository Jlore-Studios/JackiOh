--!strict
local J = require("@jackioh")

-- "Aura: Your Units have Cloaked." (R1043): Knowledge Breaker's Aura.
local function cloak_units(ctx: J.HookArgs)
	return {
		J.cloak_hand_aura({ type = "Unit" }),
	}
end

-- "Summon {traps} random (Radiant) Traps. Destroy (Radiant: Exile) every other CN or KY permanent."
-- `J.TRAP_TYPES` is typed `{string}`, not `{CardType}`, hence the cast (as Core #67 does).
local function prime(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local effects: { J.EffectCall } = {}
			for _ = 1, ctx:param("traps") do
				local args: J.SummonRandomArgs = {
					query = { type = J.TRAP_TYPES :: any },
					player = "self",
				}
				if radiant then
					args.radiant = true
				end
				effects[#effects + 1] = J.summon_random(args)
			end
			local scope: J.BoardScope = {
				side = "any",
				rows = { "units", "backrow" },
				tags = { "CN", "KY" },
				excludeSelf = true,
			}
			if radiant then
				effects[#effects + 1] = J.exile_all(scope)
			else
				effects[#effects + 1] = J.destroy_all(scope)
			end
			return effects
		end,
		aura = cloak_units,
	}
end

return {
	base = prime(false),
	radiant = prime(true),
}
