--!strict

-- Both restrictions are static flags the engine reads everywhere: can't be attacked (§6.1, §4.2
-- step 2) and #65.1's `neverDefense`. The same script on both faces: the Radiant face differs only
-- in its stats and Divine Shield, both catalog data.
local base = {
	staticFlags = {
		neverDefense = true,
		cantBeAttacked = true,
	},
}

return {
	base = base,
	radiant = "base",
}
