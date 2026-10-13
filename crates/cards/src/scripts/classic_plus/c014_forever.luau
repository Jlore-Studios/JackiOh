--!strict
local J = require("@jackioh")

local function stamp_next_spell(ctx: J.EffectContext): { J.EffectCall }
	local enchantment: J.Enchantment = { kind = "returnAfterResolve", floor = ctx:param("floor") }
	return {
		J.enchant_next_spell({ enchantment = enchantment }),
	}
end

-- The Radiant face's floor 1 is its declared `floor`.
return {
	base = { cry = stamp_next_spell },
	radiant = { cry = stamp_next_spell },
}
