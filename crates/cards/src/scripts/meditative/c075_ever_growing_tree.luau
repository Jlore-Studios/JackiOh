--!strict
local J = require("@jackioh")

-- C+ #31 Fusion Lab's shape: one Activate, once a turn, fusing a random Field Spell into self (R77,
-- R102), never this card or an ingredient already in it (R387); on the Radiant face the pick goes in
-- on its Radiant face (R469).
local function tree(radiant: boolean)
	return {
		activations = {
			{
				id = "grow",
				label = "Fuse a random Field Spell into this",
				uses = 1,
				run = function(ctx: J.EffectContext)
					local into: J.FuseInto = { target = { of = "self" } }
					return {
						J.fuse_random_into({
							into = into,
							query = { type = "Field Spell" },
							radiant = radiant,
						}),
					}
				end,
			},
		},
	}
end

return {
	base = tree(false),
	radiant = tree(true),
}
