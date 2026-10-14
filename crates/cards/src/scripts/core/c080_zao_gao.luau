--!strict
local J = require("@jackioh")

-- §7: the token this card makes.
local RUSH_TOKEN = "core-t-rush"

local TOKEN_COUNT = 2

-- The faces differ only in the tokens they summon: whether they are Radiant (R276, R354), and how
-- many keywords each rolls (R21), the declared number `keywords`. The discard is the declared
-- number `discard` (R386), less being better.
local function zao_gao(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local keywords = ctx:param("keywords")
			-- R16, R354: "random" is stated, so the match rng picks and nobody is asked.
			local effects: { J.EffectCall } = {
				J.discard_random({ count = ctx:param("discard") }),
			}
			for _ = 1, TOKEN_COUNT do
				-- One Rush Token with its rolled keywords (R21, R64).
				effects[#effects + 1] = J.summon({
					defId = RUSH_TOKEN,
					radiant = radiant,
					randomKeywords = keywords,
				})
			end
			return effects
		end,
	}
end

return {
	base = zao_gao(false),
	radiant = zao_gao(true),
}
