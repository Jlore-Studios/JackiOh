--!strict
local J = require("@jackioh")

-- The three effects the match rng picks among as the Spell resolves (MD-B9, R920).
local EFFECTS = 3

-- The Radiant face learns only the opponent's cards and makes Radiant cards and Units; the numbers
-- are the same on both faces.
local function spiritually(radiant: boolean)
	local learners: J.ScopeSide = if radiant then "enemy" else "any"
	-- Left off on the base face, as the Rust left the key off.
	local made_radiant: boolean? = if radiant then true else nil
	return {
		cry = function(ctx: J.EffectContext)
			-- One uniform `int(EFFECTS)` draw: `rng_roll` answers it plus one.
			local roll = ctx:rng_roll(EFFECTS)
			if roll == 1 then
				-- Get ready to learn Chinese: the flag on every card of both hands and both
				-- libraries (the Radiant face: the opponent's only), silently (R440).
				return {
					J.translate({ scope = { side = learners, zones = { "hand", "library" } } }),
				}
			elseif roll == 2 then
				-- Transform the hand into the same random CN card, each costing (0).
				return {
					J.transform_random({
						scope = { side = "self", zones = { "hand" } },
						same = true,
						query = { tags = { "CN" } },
						costOverride = 0,
						radiant = made_radiant,
					}),
				}
			end
			-- Summon `units` random CN Units, Radiant on the Radiant face.
			local effects = {}
			for _ = 1, ctx:param("units") do
				effects[#effects + 1] = J.summon_random({
					query = { type = "Unit", tags = { "CN" } },
					radiant = made_radiant,
				})
			end
			return effects
		end,
	}
end

return {
	base = spiritually(false),
	radiant = spiritually(true),
}
