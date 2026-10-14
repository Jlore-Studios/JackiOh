--!strict
local J = require("@jackioh")

-- "Add a random Luck-based card to your hand. Give it Lucky {lucky}." MD-G1, R1444: one uniform
-- pick of the non-token cards that print Lucky or flip a coin, of every set that ships (R1420), on
-- its base face at its printed cost, hidden from the opponent (R97); the verb's `lucky` rider
-- (MD-G2, R1438) gives it Lucky {lucky} once it is in the hand.
local function pull(ctx: J.EffectContext)
	return {
		J.add_random_from_catalog({
			query = { luckBased = true },
			lucky = ctx:param("lucky"),
		}),
	}
end

return {
	base = { cry = pull },
	-- The same pull as an Activate (R384): once a turn, in your main phase, at no cost, the turn it
	-- is played included. It has no Cry.
	radiant = {
		activations = {
			{
				id = "pull",
				label = "Add a random Luck-based card to your hand",
				uses = 1,
				run = pull,
			},
		},
	},
}
