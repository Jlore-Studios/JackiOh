--!strict
local J = require("@jackioh")

-- A spell's script is its `cry` hook (§10.9). §7: base gains 1, radiant 2 — the declared
-- number `mana` (R386), read off the face that is running.
local function coin()
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.gain_mana({ amount = ctx:param("mana") }),
			}
		end,
	}
end

return {
	base = coin(),
	radiant = coin(),
}
