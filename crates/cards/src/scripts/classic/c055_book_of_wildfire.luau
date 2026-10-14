--!strict
local J = require("@jackioh")

-- The engine's Book-swap hand trigger (`book_swap_trigger`, R671), written out: its id is the one
-- `book_swap::granted_hand_triggers` looks for (`BOOK_SWAP_TRIGGER_ID`). On its owner's `turnEnded`
-- in hand, the card becomes a random other Book on the same face; a swapped Book names the card that
-- started the swap from its `swapsBook` enchantment, and Wildfire itself is that card.
local BOOK_SWAP_TRIGGER_ID = "book-swap"

-- `book_swap_source_of`: the card that started the swap, off the enchantment it carries.
local function book_swap_source_of(card: any): string?
	if card.enchantments == nil then
		return nil
	end
	for _, entry: any in ipairs(card.enchantments) do
		if entry.kind == "swapsBook" then
			return entry.from
		end
	end
	return nil
end

local book_swap = {
	id = BOOK_SWAP_TRIGGER_ID,
	on = { "turnEnded" },
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		local me = ctx:live_self()
		if me == nil then
			return {}
		end
		if me.zone.z ~= "hand" then
			return {}
		end
		if event.type ~= "turnEnded" or event.player ~= ctx:controller() then
			return {}
		end
		local from = book_swap_source_of(me) or me.defId
		return { J.swap_book({ instanceId = me.id, from = from }) }
	end,
}

-- "Deal N damage" with no target named is targeted (§8 Conventions): one declared pick (R81) of any
-- unit or hero, either side, and one §4.4 damage instance (`param`, R386).
local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "hero" } },
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") }),
		}
	end,
	handTriggers = { book_swap },
}

-- The same script: the Radiant face differs in its declared damage (8), read through `param`, and in
-- the face of the Book it becomes, which the swap reads off the card.
return {
	base = base,
	radiant = "base",
}
