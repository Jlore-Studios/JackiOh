--!strict
local J = require("@jackioh")

-- One function under a different hook on each face (#871): the base face holds `hook`
-- under `cry`, the Radiant face only under `death`. An adapter looks its function up
-- under the key it was built for, so the faces cannot share one: each face's adapter
-- runs its own face's function, and its named `each_card`, whatever face the context
-- names.
local function hook(_ctx: J.EffectContext)
	return {
		J.for_each_card({ cards = { "lx-target" }, each = "each_card" }),
	}
end

return {
	base = {
		cry = hook,
		each_card = function(id: string)
			return { J.remember({ key = "other_keys_face", value = "base:" .. id }) }
		end,
	},
	radiant = {
		death = hook,
		each_card = function(id: string)
			return { J.remember({ key = "other_keys_face", value = "radiant:" .. id }) }
		end,
	},
}
