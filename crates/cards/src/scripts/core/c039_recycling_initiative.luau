--!strict
local J = require("@jackioh")

-- The step the end-of-turn delayed effect re-enters (§10.6: `resume[step]`), through the `resume`
-- table alone (`hook = "resume"`, R126).
local COPY_STEP = "copies"
local RESUME_HOOK = "resume"

-- The one thing the continuation captures: which play was this card's own (R71's "every other").
local SELF_KEY = "selfId"

-- The play that was this card's own. The Cry captures the id in `data` and the delay stores the same
-- instance, so the continuation's `self_` is that card while it exists. Once it has ceased to exist
-- no `self_` comes back, and its id drops out below anyway: R86 skips an id whose instance is gone.
local function excluded_id(ctx: J.EffectContext): string?
	local me = ctx:self_()
	if type(me) == "table" then
		return me.id
	end
	return nil
end

-- R71: the log is read here, when the delayed effect runs, so later plays are in it, in play order.
-- R86: an id whose instance no longer exists is skipped rather than fizzled on.
-- R133: the log holds one entry per play, so a card played, bounced and replayed is two entries and
-- one card. "Every other card you played this turn" is the set of cards, not the list of plays, so
-- each id yields one copy; deduping by id leaves R86's skip untouched.
--
-- `radiant` is what the face does to each copy: base, a plain copy at its printed price, radiant
-- flag kept (R57); radiant, a Radiant copy that costs (1) less, the declared number `discount`
-- (R386) as the card stands in exile, or the printed one once it has ceased to exist (R127).
local function copies_of_other_plays(ctx: J.EffectContext, radiant: boolean): { J.EffectCall }
	local selfId = excluded_id(ctx)
	local discount = 0
	if radiant then
		discount = ctx:param("discount")
	end
	local out: { J.EffectCall } = {}
	local seen: { [string]: boolean } = {}

	for _, id in ipairs(ctx:played_ids_this_turn(ctx:controller())) do
		if id ~= selfId and not seen[id] then
			seen[id] = true
			local card = ctx:find_instance(id)
			if type(card) == "table" then
				local args: J.AddToHandArgs = {
					defId = card.defId,
					player = "self",
					-- R57: a fresh copy carries the radiant flag and nothing else; the radiant face sets it.
					radiant = radiant or card.radiant,
				}
				if discount ~= 0 then
					args.costMod = -discount
				end
				-- R1300: a copy of a Chinese card is Chinese.
				if card.chinese == true then
					args.chinese = true
				end
				out[#out + 1] = J.add_to_hand(args)
			end
		end
	end

	return out
end

local function cry(ctx: J.EffectContext): { J.EffectCall }
	local args: J.DelayArgs = {
		at = { phase = "end", player = "self" },
		step = COPY_STEP,
		hook = RESUME_HOOK,
	}
	local me = ctx:self_()
	if type(me) == "table" then
		args.data = { [SELF_KEY] = me.id }
	end
	return {
		-- Armed before the exile, so the continuation records this instance while it still exists.
		J.delay(args),
		-- "Exile this on play."
		J.exile({ target = { of = "self" } }),
	}
end

local base = {
	cry = cry,
	-- R62's continuation, registered once: the `resume` step table the `delay` above names.
	resume = {
		[COPY_STEP] = function(ctx: J.EffectContext)
			return copies_of_other_plays(ctx, false)
		end,
	},
}

local radiant = {
	cry = cry,
	resume = {
		[COPY_STEP] = function(ctx: J.EffectContext)
			return copies_of_other_plays(ctx, true)
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
