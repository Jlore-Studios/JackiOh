--!strict

-- The whole card is one replacement, declared as data (B5 E5): at "targeted", from its
-- controller's hand, answering only a Spell's targeting (R651), the engine summons it into the
-- leftmost open unit zone (R64) and moves the pick to it. No `when`, no follow-up.
local base = {
	replacements = {
		{
			id = "joro",
			on = "targeted",
			where = "hand",
			instead = { interpose = true },
			by = "spell",
		},
	},
}

-- Same script: the Radiant face differs only in the catalog's Indestructible.
return {
	base = base,
	radiant = "base",
}
