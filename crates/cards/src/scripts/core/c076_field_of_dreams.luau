--!strict
local J = require("@jackioh")

-- R31: the card the hand is replaced with — #72 Reminisce, by catalog id.
local REMINISCE = "core-072"

-- The two faces differ only in whether the copies are Radiant (§5.2, R74).
local function field_of_dreams(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			-- N, the caller's hand size at resolution: §10.5 step 4 has already moved this card out.
			local count = ctx:zone_count(ctx:controller(), "hand")
			local effects: { J.EffectCall } = {}
			-- R31: the replaced hand goes to the graveyard, where Reminisce can find it.
			effects[#effects + 1] = J.discard_hand({ player = "self" })
			for _ = 1, count do
				effects[#effects + 1] = J.add_to_hand({ defId = REMINISCE, radiant = radiant })
			end
			-- "exile this": the spell leaves from `resolving` and never reaches the graveyard.
			effects[#effects + 1] = J.exile({ target = { of = "self" } })
			return effects
		end,
	}
end

return {
	base = field_of_dreams(false),
	radiant = field_of_dreams(true),
}
