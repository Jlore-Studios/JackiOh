--!strict
local J = require("@jackioh")

-- "Recruit every permanent in your deck": the engine's `recruitAll`, top down.
local function cry(ctx: J.EffectContext)
	return { J.recruit_all({}) }
end

-- The base clause is a replacement on this card's own move to its graveyard: to the bottom of
-- the deck instead (a full deck turns it away, R80).
local base = {
	cry = cry,
	replacements = {
		{
			id = "pile-on",
			on = "toGraveyard",
			where = "self",
			instead = { to = "bottomOfLibrary" },
		},
	},
}

-- The Radiant face drops the return (SPEC §8.6 row 60): the same Recruit, and this Spell goes to the
-- graveyard as any Spell does.
local radiant = {
	cry = cry,
}

return {
	base = base,
	radiant = radiant,
}
