--!strict
local J = require("@jackioh")

-- The Spell type only, never a Field Spell: non-token Spells of every set that ships (R380, R1420),
-- never this card (R387), repeats allowed (R60).
local function spell_basket(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_random_from_catalog({
					query = { type = "Spell" },
					count = ctx:param("cards"),
					radiant = radiant,
				}),
			}
		end,
	}
end

return {
	base = spell_basket(false),
	radiant = spell_basket(true),
}
