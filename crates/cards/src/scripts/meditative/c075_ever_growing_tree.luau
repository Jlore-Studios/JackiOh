--!strict
local J = require("@jackioh")

-- Patch v0.3.9 (#831): "Choose 1 of 2 random Field Spells", a Discover of two (R831012).
local TREE_OFFERS = 2

-- The resume step the choice re-enters (§10.6).
local PICKED = "picked"

-- C+ #31 Fusion Lab's shape: one Activate, once a turn, fusing a Field Spell into self (R77, R102),
-- never this card or an ingredient already in it (R387), the Tree's cost kept; on the Radiant face
-- the pick goes in on its Radiant face (R469). The two on offer are random; the one fused is chosen.
local function tree(radiant: boolean)
	local prompt = "Ever Growing Tree: choose a Field Spell to fuse into it"
	if radiant then
		prompt = "Ever Growing Tree: choose a Radiant Field Spell to fuse into it"
	end
	return {
		activations = {
			{
				id = "grow",
				label = "Fuse one of 2 random Field Spells into this",
				uses = 1,
				run = function(ctx: J.EffectContext)
					return {
						J.discover_from_catalog({
							step = PICKED,
							count = TREE_OFFERS,
							query = { type = "Field Spell" },
							prompt = prompt,
						}),
					}
				end,
			},
		},
		resume = {
			[PICKED] = function(ctx: J.EffectContext)
				local picked = ctx:chosen_options()[1]
				local me = ctx:self_()
				if picked == nil or type(me) ~= "table" then
					return {}
				end
				local args: J.FuseCardsArgs = { defIds = { picked }, targetInstanceId = me.id, keepCost = true }
				if radiant then
					args.radiantIngredients = true
				end
				return { J.fuse_cards(args) }
			end,
		},
	}
end

return {
	base = tree(false),
	radiant = tree(true),
}
