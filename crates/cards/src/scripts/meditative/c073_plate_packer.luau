--!strict
local J = require("@jackioh")

-- +{per}/+{per} for each Armor on the field, heroes' included (R1064), read as the Cry resolves.
-- One hook on both faces: the Radiant face's 2 is its declared `per`, read off the running face.
local function cry(ctx: J.EffectContext)
	local gain = ctx:armor_on_field() * ctx:param("per")
	if gain == 0 then
		return {}
	end
	return {
		J.buff({ target = { of = "self" }, attack = gain, health = gain }),
	}
end

return {
	base = { cry = cry },
	radiant = { cry = cry },
}
