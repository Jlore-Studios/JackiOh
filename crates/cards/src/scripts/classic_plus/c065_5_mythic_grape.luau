--!strict
local J = require("@jackioh")

-- §8.7: "They cost (0)": the declared number `setCost` (R386), less being better.
local function mythic_grape(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.replace_hand_with_random({
					query = { rarity = "Mythic" },
					costOverride = ctx:param("setCost"),
					radiant = if radiant then true else nil,
				}),
			}
		end,
	}
end

return {
	base = mythic_grape(false),
	radiant = mythic_grape(true),
}
