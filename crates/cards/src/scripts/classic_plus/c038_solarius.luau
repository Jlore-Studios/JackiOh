--!strict
local J = require("@jackioh")

local SOLARIUS_PRIME = "classicplus-038-1"

-- One fresh C+ #38.1 at a random position in the deck, R80's cap turning it away.
local PRIMES = 1

local base = {
	death = function(ctx: J.EffectContext)
		return {
			J.shuffle_into({ defId = SOLARIUS_PRIME, count = PRIMES, radiant = false }),
		}
	end,
}

local radiant = {
	death = function(ctx: J.EffectContext)
		return {
			J.shuffle_into({ defId = SOLARIUS_PRIME, count = PRIMES, radiant = true }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
