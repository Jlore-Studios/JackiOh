--!strict

-- Both faces summon their joiners (ME-ATTACKSUMMON, R1203); the count is the catalog `joiners`
-- param, and the Radiant face's First Strike is catalog data beside it.
local base = {
	staticFlags = { attackJoiners = true },
}

return {
	base = base,
	radiant = "base",
}
