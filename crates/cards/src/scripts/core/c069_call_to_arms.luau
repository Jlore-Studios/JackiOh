--!strict
local J = require("@jackioh")

-- §8.3: "Recruit 3", the same on both faces.
local RECRUITS = 3

local function units_costing(max: number): J.RecruitFilter
	return { type = "Unit", costRange = { max = max } }
end

-- The cost limit is the whole of the radiant text: the declared number `costLimit` (R386), read off
-- the face that is running.
local base = {
	cry = function(ctx: J.EffectContext)
		local filter = units_costing(ctx:param("costLimit"))
		local out = {}
		for _ = 1, RECRUITS do
			out[#out + 1] = J.recruit({ filter = filter, player = "self" })
		end
		return out
	end,
}

return {
	base = base,
	radiant = "base",
}
