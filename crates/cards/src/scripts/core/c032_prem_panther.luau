--!strict
local J = require("@jackioh")

-- §8 row 32: "draw 2 for each Unit that attack destroyed", the 2 the declared number `draw` (R386).
-- The combat's facts (`afterAttackOf`) name the Units whose lethal hit it dealt there; survival is not
-- asked (R212).
local function after_attack(ctx: J.EffectContext)
	local facts = ctx:after_attack_of()
	if facts == nil then
		return {}
	end
	if #facts.destroyedIds == 0 then
		return {}
	end
	return {
		J.draw({ count = ctx:param("draw") * #facts.destroyedIds }),
	}
end

-- "Rush, Cleave / the same": the keyword list is the radiant face's, the text is unchanged.
return {
	base = { afterAttack = after_attack },
	radiant = { afterAttack = after_attack },
}
