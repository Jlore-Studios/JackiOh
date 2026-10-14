--!strict
local J = require("@jackioh")

-- #65.1, the token this card defines (§7, §8.3).
local SPIKEY_PILLOW = "core-065-1"

-- The option names, worded as §8.3's two cells list them and in that order. These are prompt
-- options, not declared modes (R81), so they reach the client through `state.pending.options`.
local EXILE_BOTTOM = "exile bottom"
local SUMMON_PILLOW = "summon Spikey Pillow"
local NOTHING = "nothing"

-- The steps of the `resume` table (§10.6); the radiant face uses both.
local STEP_FIRST = "firstPick"
local STEP_SECOND = "secondPick"

-- "lose 3", named with the health it loses: the declared number `loss` (R386), less being better,
-- as the card stands when the prompt opens.
local function lose_option(ctx: J.EffectContext): string
	return "lose " .. tostring(ctx:param("loss"))
end

local function base_options(ctx: J.EffectContext): { string }
	return { EXILE_BOTTOM, lose_option(ctx), SUMMON_PILLOW }
end

local function radiant_options(ctx: J.EffectContext): { string }
	return { NOTHING, EXILE_BOTTOM, lose_option(ctx), SUMMON_PILLOW }
end

-- What one answered option does. "nothing" and an answer that named no option are both the empty
-- list: the trigger still fired and the card stays on the field.
local function effects_for(ctx: J.EffectContext, option: string?): { J.EffectCall }
	if option == EXILE_BOTTOM then
		return { J.exile_bottom_of_library({ player = "self" }) }
	end
	if option == SUMMON_PILLOW then
		return { J.summon({ defId = SPIKEY_PILLOW }) }
	end
	-- R18: lost health, not damage.
	if option ~= nil and option == lose_option(ctx) then
		return { J.lose_health({ player = "self", amount = ctx:param("loss") }) }
	end
	return {}
end

-- The answered pick: §10.6 delivers a mode selection in `ctx.targets`, which is what this reads.
local function pick_of(ctx: J.EffectContext): string?
	return ctx:chosen_options()[1]
end

-- §6.2: Quickdraw is the `quickdraw` static flag, which `setup.rs` reads; both faces carry it.
local FLAGS = { quickdraw = true }

-- Each face's start of turn opens one mode prompt whose answer re-enters a named `resume` step. The
-- prompt effect is LAST in every list, correct under `apply_resumable` (parks a tail) and
-- `resolve::apply_effects` (does not; `turn.rs` runs start-of-turn hooks with it).
local base = {
	staticFlags = FLAGS,
	startOfTurn = function(ctx: J.EffectContext)
		return {
			J.choose_mode({
				options = base_options(ctx),
				step = STEP_FIRST,
				prompt = "Masochism Mask: choose one",
			}),
		}
	end,
	resume = {
		[STEP_FIRST] = function(ctx: J.EffectContext)
			return effects_for(ctx, pick_of(ctx))
		end,
	},
}

local radiant = {
	staticFlags = FLAGS,
	startOfTurn = function(ctx: J.EffectContext)
		return {
			J.choose_mode({
				options = radiant_options(ctx),
				step = STEP_FIRST,
				prompt = "Masochism Mask: choose twice (1 of 2)",
			}),
		}
	end,
	resume = {
		-- The first pick resolves, then the second prompt opens — the same option may be picked again,
		-- because these are two separate prompts and the "picked twice" check in `prompts.rs` is
		-- per-prompt.
		[STEP_FIRST] = function(ctx: J.EffectContext)
			local effects = effects_for(ctx, pick_of(ctx))
			effects[#effects + 1] = J.choose_mode({
				options = radiant_options(ctx),
				step = STEP_SECOND,
				prompt = "Masochism Mask: choose twice (2 of 2)",
			})
			return effects
		end,
		[STEP_SECOND] = function(ctx: J.EffectContext)
			return effects_for(ctx, pick_of(ctx))
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
