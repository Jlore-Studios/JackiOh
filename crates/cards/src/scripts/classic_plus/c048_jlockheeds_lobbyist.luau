--!strict
local J = require("@jackioh")

-- The Death's pool is one tag (R278); `add_random_from_catalog` leaves this card out by its def id
-- (R387). "It costs (0)": the declared number `setCost` (R386), on both faces.
local JLOCKEED: J.CatalogQueryArgs = { tags = { "Jlockeed" } }

local base = {
	-- Can't be in Defense Position: #65.1's position validator flag (R20).
	staticFlags = { neverDefense = true },
	death = function(ctx: J.EffectContext)
		return {
			J.add_random_from_catalog({
				query = JLOCKEED,
				costOverride = ctx:param("setCost"),
			}),
		}
	end,
}

-- The Radiant face drops the Defense restriction, and the card it adds is Radiant.
local radiant = {
	death = function(ctx: J.EffectContext)
		return {
			J.add_random_from_catalog({
				query = JLOCKEED,
				costOverride = ctx:param("setCost"),
				radiant = true,
			}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
