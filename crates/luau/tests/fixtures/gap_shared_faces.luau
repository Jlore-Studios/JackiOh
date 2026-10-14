--!strict
local J = require("@jackioh")

-- Gap 14: one function under two hooks across both faces. The base and Radiant
-- faces are separate tables holding the one `death`, so the loader shares one
-- adapter for both (`classic-082`, `classicplus-047`, whose faces' params
-- differ; the shared function reads `param` off the running face).
local function death(_ctx: J.EffectContext)
	return {
		J.gain_mana({ amount = 0 }),
	}
end

-- The shared `cry` resolves its named `each_card` on the RUNNING face: each
-- face remembers its own name, so a run on one face never fires the other's
-- (`classicplus-012-7`'s shape: one `cry`, per-face closers).
local function cry(_ctx: J.EffectContext)
	return {
		J.for_each_card({ cards = { "lx-target" }, each = "each_card" }),
	}
end

return {
	base = {
		death = death,
		cry = cry,
		each_card = function(id: string)
			return { J.remember({ key = "gap14_face", value = "base:" .. id }) }
		end,
	},
	radiant = {
		death = death,
		cry = cry,
		each_card = function(id: string)
			return { J.remember({ key = "gap14_face", value = "radiant:" .. id }) }
		end,
	},
}
