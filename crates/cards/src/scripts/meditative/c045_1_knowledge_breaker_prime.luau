--!strict
local J = require("@jackioh")

-- ME-ALTPLAY: "You may play your Units face-down as Animated Field Traps that reveal at the start
-- of your next turn".
local function face_down_units(ctx: J.HookArgs)
	return {
		{ units = true },
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
		faceDownPlay = face_down_units,
	}
end

return {
	base = prime(false),
	radiant = prime(true),
}
