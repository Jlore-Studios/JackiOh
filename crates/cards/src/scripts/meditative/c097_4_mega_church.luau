--!strict
local J = require("@jackioh")

-- §8.8: "Tribute 5".
local TRIBUTE_COST = 5
-- Costs (1) or less on either side.
local TRIBUTE_CHEAP = 1
-- Activate is once a turn.
local USES = 1

-- WIDE-TRIBUTE (R1282) is the static flags the legal Tribute sets read. Activate (R384): one
-- declared enemy permanent acting on the field, stolen (§6.3, R15, R171), then made Radiant on the
-- Radiant face.
local function church(radiant: boolean)
	return {
		staticFlags = { tribute = TRIBUTE_COST, tributeCheap = TRIBUTE_CHEAP },
		activations = {
			{
				id = "church-steal",
				label = "Steal an enemy permanent",
				uses = USES,
				targets = {
					{ kind = "target", min = 1, max = 1, filter = { side = "enemy", of = { "unit", "backrow" } } },
				},
				run = function(ctx: J.EffectContext)
					local effects = { J.steal({ target = { of = "chosen" } }) }
					if radiant then
						effects[#effects + 1] = J.set_radiant({ target = { of = "chosen" } })
					end
					return effects
				end,
			},
		},
	}
end

return {
	base = church(false),
	radiant = church(true),
}
