--!strict
local J = require("@jackioh")

-- What the Auctioneer keeps once it has revealed.
local ACTIVATED = "activated"

local function has_activated(ctx: J.EffectContext): boolean
	return ctx:recalled(ACTIVATED) == true
end

-- The answer this play gets: "activate" (the {plays}th play of a turn), "sale" (once activated), or nil.
local function answer(ctx: J.EffectContext, event: J.GameEvent): string?
	if event.type ~= "cardPlayed" then
		return nil
	end
	local reached = ctx:cards_played_this_turn(event.player) == ctx:param("plays")
	if has_activated(ctx) then
		return "sale"
	end
	if reached then
		return "activate"
	end
	return nil
end

local auction = {
	id = "auctioneer",
	on = { "cardPlayed" },
	when = function(ctx: J.EffectContext, event: J.GameEvent)
		return answer(ctx, event) ~= nil
	end,
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		local kind = answer(ctx, event)
		if kind == "activate" then
			return {
				J.remember({ key = ACTIVATED, value = true }),
				J.animate({}),
			}
		end
		if kind == "sale" then
			-- B3.1 rule 4: every firing of an Animated trap ends with its animating, so one stuck in the
			-- backrow for want of a zone steps into a unit zone as soon as one is open; a Unit stays put.
			return {
				J.draw({ count = ctx:param("draw") }),
				J.damage({ to = { of = "enemyHero" }, amount = ctx:param("damage") }),
				J.animate({}),
			}
		end
		return {}
	end,
}

-- The same script: the Radiant face's 2nd play and 4 damage are its declared numbers, which `param`
-- reads off the face; its 8/8 body is printed on the catalog face.
return {
	base = {
		triggers = { auction },
	},
	radiant = "base",
}
