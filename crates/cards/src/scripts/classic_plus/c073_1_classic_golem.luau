--!strict
local J = require("@jackioh")

-- §8.7: "a random Classic or Classic+ Unit" — non-token Units of the two sets the text names (R380).
local function classic_units(): J.CatalogQueryArgs
	return { type = "Unit", set = { "Classic", "Classic+" } }
end

-- A hero target is `hero-<player>` in the combat's facts.
local HERO_TARGET = "hero-"

-- R424: the transform follows the combat of an attack the Golem declared on a Unit (not a forced one,
-- R53); a Golem that left the field transforms into nothing; killing the defender (R42) readies the new
-- Unit. Each face holds its own hook: `radiant` is the face's own upvalue.
local function after_it_attacks(radiant: boolean)
	return function(ctx: J.EffectContext)
		local combat = ctx:after_attack_of()
		if combat == nil then
			return {}
		end
		if combat.forced or not combat.survived or string.sub(combat.targetId, 1, #HERO_TARGET) == HERO_TARGET then
			return {}
		end
		local destroyed = false
		for _, id in ipairs(combat.destroyedIds) do
			if id == combat.targetId then
				destroyed = true
			end
		end
		return {
			J.transform_random({
				target = { of = "self" },
				query = classic_units(),
				radiant = if radiant then true else nil,
				readyToAttack = destroyed,
			}),
		}
	end
end

-- The Radiant face transforms into a Radiant Unit; its 20/20 and keywords are its catalog face.
return {
	base = { afterAttack = after_it_attacks(false) },
	radiant = { afterAttack = after_it_attacks(true) },
}
