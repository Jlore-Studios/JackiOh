--!strict
local J = require("@jackioh")

local PRIME = "meditative-091-1"

-- §8.8: "Tribute 1".
local TRIBUTE_COST = 1

-- "Death: Shuffle a Windfurious Prime into your deck": one.
local PRIMES = 1

-- ME-ATTACKSUMMON (R1202): a Unit from hand makes each attack instead. The base face bounces the
-- substitute afterwards if it survives; the Radiant face keeps it, and its Prime is Radiant.
local function windfast(radiant_prime: boolean)
	return {
		staticFlags = {
			tribute = TRIBUTE_COST,
			attackFromHand = true,
			bounceAttacker = if radiant_prime then nil else true,
		},
		death = function(ctx: J.EffectContext)
			return {
				J.shuffle_into({
					defId = PRIME,
					count = PRIMES,
					radiant = radiant_prime,
				}),
			}
		end,
	}
end

return {
	base = windfast(false),
	radiant = windfast(true),
}
