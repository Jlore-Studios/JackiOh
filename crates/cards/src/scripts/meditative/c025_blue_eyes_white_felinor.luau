--!strict

-- Tribute 2 on both faces; the keywords come from the catalog.
local TRIBUTE_COST = 2

-- The Radiant face keeps the Tribute and adds Immutable in the catalog; the script is shared.
local base = {
	staticFlags = { tribute = TRIBUTE_COST },
}

return {
	base = base,
	radiant = "base",
}
