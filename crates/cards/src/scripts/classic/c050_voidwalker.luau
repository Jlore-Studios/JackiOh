--!strict
local J = require("@jackioh")

-- The aura is a replacement at "would go to a graveyard" while Voidwalker is on the field:
-- `{ id: "voidwalker", on: "toGraveyard", instead: { to: "exile" } }`, with the face's `when`.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.exile_matching({ zones = { "graveyard" }, player = "self" }),
			J.exile_matching({ zones = { "graveyard" }, player = "enemy" }),
		}
	end,
	replacements = {
		{
			id = "voidwalker",
			on = "toGraveyard",
			instead = { to = "exile" },
		},
	},
}

-- The Radiant face judges by owner: only cards its controller's opponent owns are exiled.
local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.exile_matching({ zones = { "graveyard" }, player = "enemy" }),
		}
	end,
	replacements = {
		{
			id = "voidwalker",
			on = "toGraveyard",
			when = function(ctx: J.ReplacementContext)
				local event = ctx:event()
				return event.moment == "toGraveyard" and event.owner ~= ctx:controller()
			end,
			instead = { to = "exile" },
		},
	},
}

return {
	base = base,
	radiant = radiant,
}
