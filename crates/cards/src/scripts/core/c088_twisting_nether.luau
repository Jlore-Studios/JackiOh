--!strict
local J = require("@jackioh")

-- The radiant face's choice, in the order §8's cell lists it: enemy only, or everything.
local MODE_ENEMY = "enemy"
local MODE_ALL = "all"

-- Which side the destroy reaches. The base face has no choice to read and always hits both sides;
-- the radiant face hits what the play named, and a play carrying no mode fizzles rather than
-- guessing — the spell still counts as played (§6.3, §8 Conventions).
local function side_for(modeName: string?, modal: boolean): J.ScopeSide?
	if not modal then
		return "any"
	end
	if modeName == MODE_ENEMY then
		return "enemy"
	end
	if modeName == MODE_ALL then
		return "any"
	end
	return nil
end

-- `modal` is the whole of the radiant text.
local function cry(ctx: J.EffectContext, modal: boolean): { J.EffectCall }
	local side = side_for(ctx:chosen_options()[1], modal)
	if side == nil then
		return {}
	end
	-- §6.3: a permanent is a Unit, Field Spell, Trap or Field Trap, so both rows.
	return { J.destroy_all({ side = side, rows = { "units", "backrow" } }) }
end

local base = {
	cry = function(ctx: J.EffectContext)
		return cry(ctx, false)
	end,
}

local radiant = {
	modes = {
		{ kind = "mode", options = { MODE_ENEMY, MODE_ALL } },
	},
	cry = function(ctx: J.EffectContext)
		return cry(ctx, true)
	end,
}

return {
	base = base,
	radiant = radiant,
}
