--!strict
local J = require("@jackioh")

-- One damage to every enemy unit, dealt through `for_each_card` with both
-- callbacks: the list comes from a reader, the blow from a function of the id.
local base = {
	cry = function(ctx)
		return {
			J.for_each_card({
				cards = function(inner)
					local ids = {}
					for _, unit in ipairs(inner:active_units_of(inner:opponent_of(inner:controller()))) do
						ids[#ids + 1] = unit.id
					end
					return ids
				end,
				each = function(id)
					return { J.damage({ to = { of = "instance", instanceId = id }, amount = 1 }) }
				end,
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
