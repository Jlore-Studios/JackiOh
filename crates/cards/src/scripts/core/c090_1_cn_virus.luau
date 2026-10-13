--!strict
local J = require("@jackioh")

-- R350: the step the end-of-turn delayed effect re-enters (§10.6: `resume[step]`).
local COPIES_STEP = "copies"
local RESUME_HOOK = "resume"

local function cry(ctx: J.EffectContext): { J.EffectCall }
	return {
		J.damage({ to = { of = "selfHero" }, amount = ctx:param("damage") }),
		-- R350: the copies at the end of the turn this is cast on, whoever's turn that is.
		J.delay({
			at = { phase = "end", player = "turn" },
			step = COPIES_STEP,
			hook = RESUME_HOOK,
		}),
	}
end

-- R57: fresh copies with the flag of the face that was cast; R80 and R316 at a full library.
local function copies(ctx: J.EffectContext): { J.EffectCall }
	local args: J.ShuffleIntoArgs = {
		defId = "core-090-1",
		count = ctx:param("copies"),
		player = "self",
		radiant = ctx:radiant(),
	}
	local me = ctx:self_()
	if type(me) == "table" then
		args.copyOf = me.id
	end
	return { J.shuffle_into(args) }
end

-- One shared `cry` and one shared step: the faces differ only in their declared numbers
-- (`damage`, `copies`), which `param` reads off the running face, and `radiant` off `ctx`.
local base = {
	staticFlags = { castOnDraw = true },
	cry = cry,
	resume = {
		[COPIES_STEP] = copies,
	},
}

local radiant = {
	staticFlags = { castOnDraw = true },
	cry = cry,
	resume = {
		[COPIES_STEP] = copies,
	},
}

return {
	base = base,
	radiant = radiant,
}
