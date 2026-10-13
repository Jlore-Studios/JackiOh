--!strict
local J = require("@jackioh")

-- §7's shared Rush Token, 3/3 with Rush.
local RUSH_TOKEN = "core-t-rush"

-- "Summon 3 Rush Tokens" on the Radiant face: the declared number `tokens` (R386), tuned on the
-- Radiant face only — the base face's "a Rush Token" is 1 and never moves (R749), so both faces read
-- the one number.
local function conjure(ctx: J.EffectContext): { J.EffectCall }
	local keywords = ctx:param("keywords")
	local effects = {}
	for _ = 1, ctx:param("tokens") do
		effects[#effects + 1] = J.summon({ defId = RUSH_TOKEN, randomKeywords = keywords })
	end
	return effects
end

return {
	base = { cry = conjure },
	radiant = { cry = conjure },
}
