--!strict
local J = require("@jackioh")

-- R81: the declared direction, enumerated by `legal_actions` and answered in the `play` action.
local LEFT = "left"
local RIGHT = "right"

-- The declared pick, narrowed rather than cast. `chosen_options` reads a prompt answer's mode first
-- and `ctx.modes` second, so one reader covers both an ordinary play and an Echo repeat of this Cry.
-- R90 keeps the play legal with the answers that exist, so a play that carried no direction fizzles
-- here instead of guessing a side for the whole board.
local function direction_of(ctx: J.EffectContext): J.RotationDirection?
	local picked = ctx:chosen_options()[1]
	if picked == LEFT then
		return LEFT
	end
	if picked == RIGHT then
		return RIGHT
	end
	return nil
end

-- §8: one step around each ring, in the direction the play declared. The radiant difference is
-- `ctx.radiant`, which the rotation subsystem reads for itself (R14's "radiant bounces go to the
-- card's controller's hand", R747), so both faces share the declaration and the hook.
local base = {
	modes = {
		{ kind = "direction", options = { LEFT, RIGHT } },
	},
	cry = function(ctx: J.EffectContext)
		local direction = direction_of(ctx)
		if direction == nil then
			return {}
		end
		return { J.rotate({ direction = direction }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
