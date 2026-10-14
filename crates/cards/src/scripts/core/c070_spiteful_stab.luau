--!strict
local J = require("@jackioh")

local ID = "core-070"

-- One face's numbers: the flat damage, the health step, and what each card in your exile adds.
type Stab = {
	base: number,
	step: number,
	perExiled: number,
}

-- §8: "Deal 2 …, +1 per full 5 health …, +1 per card …"; radiant "Deal 4 …, per full 3 …, +2 per
-- card …": the declared numbers `damage`, `healthStep` and `perExile` (R386), as the card stands.
local function numbers_of(ctx: any): Stab
	return {
		base = ctx:param("damage"),
		step = ctx:param("healthStep"),
		perExiled = ctx:param("perExile"),
	}
end

-- §10.9: a hook may read state to compute an effect's arguments; it never writes. Both reads go
-- through the engine's read-only board surface (`hero_of`, `zone_count`, BUILD M3-T1).
-- R72: missing health floors at 0 (§2's 30, `HERO_HEALTH`), and the exile is the controller's OWN
-- pile. `missing` is never negative and `step` positive, so `J.div` floors.
local function stab_amount(ctx: any, controller: J.PlayerId, face: Stab): number
	local missing = math.max(J.config.HERO_HEALTH - ctx:hero_of(controller).health, 0)
	return face.base + J.div(missing, face.step) + face.perExiled * ctx:zone_count(controller, "exile")
end

local function spiteful_stab(face: J.FaceKind)
	return {
		-- §8 Conventions: every legal unit and hero on either side (R90: a bare `target` is units only).
		targets = {
			{
				kind = "target",
				min = 1,
				max = 1,
				filter = { side = "any", of = { "unit", "hero" } },
			},
		},
		cry = function(ctx: J.EffectContext)
			local amount = stab_amount(ctx, ctx:controller(), numbers_of(ctx))
			return {
				J.damage({ to = { of = "chosen" }, amount = amount }),
			}
		end,
		-- R280: the label is the face's whole text as printed, with its declared numbers as the card
		-- stands (R386).
		preview = function(ctx: J.ConditionContext)
			return {
				{
					label = ctx:fill_params(ID, face, ctx:params_view()),
					value = stab_amount(ctx, ctx:controller(), numbers_of(ctx)),
				},
			}
		end,
		-- R662: the scaling has kicked in.
		conditionMet = function(ctx: J.ConditionContext)
			local numbers = numbers_of(ctx)
			return ctx:zone() == "hand" and stab_amount(ctx, ctx:controller(), numbers) > numbers.base
		end,
	}
end

return {
	base = spiteful_stab("base"),
	radiant = spiteful_stab("radiant"),
}
