--!strict
local J = require("@jackioh")

-- N, declared with the play, "1" to `ROLLBACK_MAX_TURNS`.
local function turns(): J.ModeDecl
	local options: { string } = {}
	for n = 1, J.config.ROLLBACK_MAX_TURNS do
		options[#options + 1] = tostring(n)
	end
	return { kind = "number", options = options }
end

-- The Radiant face's parts of the board, as its play names them.
type SideEntry = { label: string, side: J.RollBackSides }
local SIDES: { SideEntry } = {
	{ label = "your side", side = "self" },
	{ label = "your opponent's side", side = "enemy" },
	{ label = "both sides", side = "both" },
}

local function back(ctx: J.EffectContext, sides: J.RollBackSides?): { J.EffectCall }
	local turnsAgo = ctx:chosen_number()
	if turnsAgo == nil or sides == nil then
		return {}
	end
	return {
		J.roll_back({ turnsAgo = turnsAgo, sides = sides }),
	}
end

-- The side the Radiant play named: the number answer parses, so only a side label matches.
local function chosen_side(ctx: J.EffectContext): J.RollBackSides?
	for _, option in ipairs(ctx:chosen_options()) do
		for _, entry in ipairs(SIDES) do
			if option == entry.label then
				return entry.side
			end
		end
	end
	return nil
end

local base = {
	modes = { turns() },
	cry = function(ctx: J.EffectContext)
		return back(ctx, "both")
	end,
}

local radiant = {
	modes = {
		turns(),
		{
			kind = "mode",
			options = { SIDES[1].label, SIDES[2].label, SIDES[3].label },
		},
	},
	cry = function(ctx: J.EffectContext)
		return back(ctx, chosen_side(ctx))
	end,
}

return {
	base = base,
	radiant = radiant,
}
