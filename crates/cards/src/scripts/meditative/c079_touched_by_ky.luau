--!strict
local J = require("@jackioh")

-- ME-HANDCAP (R1143): the cap is set before the draws, which then use the new room. The Radiant
-- face's draws are separate `draw_priced` draws whose card, if it reached the hand, becomes
-- Radiant (R596).
local function touched_by_ky(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local effects = { J.set_hand_cap({ cap = ctx:param("handSize") }) }
			local draws = ctx:param("draw")
			if radiant then
				for _ = 1, math.max(draws, 0) do
					effects[#effects + 1] = J.draw_priced({ radiant = true })
				end
			else
				effects[#effects + 1] = J.draw({ count = draws })
			end
			return effects
		end,
	}
end

return {
	base = touched_by_ky(false),
	radiant = touched_by_ky(true),
}
