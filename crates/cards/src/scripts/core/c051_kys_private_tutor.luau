--!strict
local J = require("@jackioh")

-- #51.1, the token the empty-library path adds (§8.3, §7).
local NOTEBOOK = "core-051-1"

-- §6.3 Discover: 1 of 3, so three library cards are revealed.
local REVEAL_COUNT = 3

-- The four type options, in the §8 row's printed order.
type TypeOption = "Spell" | "Unit" | "Field Spell" | "Trap"
local TYPE_OPTIONS: { TypeOption } = { "Spell", "Unit", "Field Spell", "Trap" }

-- The four cost brackets, in the §8 row's printed order.
local BRACKET_OPTIONS = { "0-1", "2", "3", "4+" }

-- The key the chosen type travels under, from the bracket step to the reveal step (§10.6).
local TYPE_KEY = "type"

-- Which card types one option covers: "Field Trap counts as Trap"; every other option is its own
-- type, so "Spell" never reaches a Field Spell and "Field Spell" never a Spell.
local function types_for(option: TypeOption): { J.CardType }
	if option == "Trap" then
		return { "Trap", "Field Trap" }
	end
	return { option }
end

-- R65's out-of-play cost range a bracket means; "4+" has no upper bound.
local function range_for(bracket: string): { min: number?, max: number? }
	if bracket == "0-1" then
		return { min = 0, max = 1 }
	end
	if bracket == "2" then
		return { min = 2, max = 2 }
	end
	if bracket == "3" then
		return { min = 3, max = 3 }
	end
	return { min = 4, max = nil }
end

-- "Your library": top card first. A unit-token card in the library leaves it only by being drawn
-- or played (R11), and "choose one to hand" is neither, so the Tutor passes over it — as the
-- reveal does — and never offers a type or a bracket only it matches.
local function library_cards(ctx: J.EffectContext): { any }
	local cards: { any } = {}
	for _, card in ipairs(ctx:zone_cards(ctx:controller(), "library")) do
		if not ctx:is_unit_token(card.id) then
			cards[#cards + 1] = card
		end
	end
	return cards
end

local function matches_type(ctx: J.EffectContext, card: any, option: TypeOption): boolean
	local type_ = ctx:def_of(card.defId).type
	for _, covered in ipairs(types_for(option)) do
		if type_ == covered then
			return true
		end
	end
	return false
end

-- R65: the library card's own cost (`effective_cost`).
local function matches_bracket(ctx: J.EffectContext, card: any, bracket: string): boolean
	local cost = ctx:effective_cost(card.id)
	local range = range_for(bracket)
	if range.min ~= nil and cost < range.min then
		return false
	end
	if range.max ~= nil and cost > range.max then
		return false
	end
	return true
end

-- A `chosen_options` answer narrowed back to the option it must be, or nil (never a cast).
local function type_of(picked: string?): TypeOption?
	for _, option in ipairs(TYPE_OPTIONS) do
		if option == picked then
			return option
		end
	end
	return nil
end

local function bracket_of(picked: string?): string?
	for _, option in ipairs(BRACKET_OPTIONS) do
		if option == picked then
			return option
		end
	end
	return nil
end

-- §10.6's captured data is JSON, so what comes back out is narrowed, not asserted.
local function captured_type(ctx: J.EffectContext): TypeOption?
	local data = ctx:data()
	if type(data) ~= "table" then
		return nil
	end
	local stored = data[TYPE_KEY]
	if type(stored) ~= "string" then
		return nil
	end
	return type_of(stored)
end

-- Step 1 (the spell's own resolution): offer the types that have a match in the library. No type
-- does — an empty library — so the Notebook is added instead.
local function start_step(ctx: J.EffectContext): { J.EffectCall }
	local library = library_cards(ctx)
	local options: { string } = {}
	for _, option in ipairs(TYPE_OPTIONS) do
		for _, card in ipairs(library) do
			if matches_type(ctx, card, option) then
				options[#options + 1] = option
				break
			end
		end
	end
	if #options == 0 then
		return {
			J.add_to_hand({ defId = NOTEBOOK }),
		}
	end
	return {
		J.choose_mode({
			options = options,
			step = "bracket",
			prompt = "Choose a card type",
		}),
	}
end

-- Step 2: the type is answered, so offer the cost brackets that have a match FOR THAT TYPE, and
-- carry the type forward — the reveal step needs it and `ctx.targets` will by then hold the
-- bracket answer instead.
local function bracket_step(ctx: J.EffectContext): { J.EffectCall }
	local type_ = type_of(ctx:chosen_options()[1])
	if type_ == nil then
		return {}
	end
	local options: { string } = {}
	for _, bracket in ipairs(BRACKET_OPTIONS) do
		for _, card in ipairs(library_cards(ctx)) do
			if matches_type(ctx, card, type_) and matches_bracket(ctx, card, bracket) then
				options[#options + 1] = bracket
				break
			end
		end
	end
	if #options == 0 then
		return {}
	end
	return {
		J.choose_mode({
			options = options,
			step = "reveal",
			data = { [TYPE_KEY] = type_ },
			prompt = "Choose a cost bracket for " .. type_,
		}),
	}
end

-- Step 3: both halves of the filter are known, so reveal three random matching library cards.
local function reveal_step(ctx: J.EffectContext): { J.EffectCall }
	local type_ = captured_type(ctx)
	local bracket = bracket_of(ctx:chosen_options()[1])
	if type_ == nil or bracket == nil then
		return {}
	end
	local range = range_for(bracket)
	local filter: J.LibraryFilter = { type = types_for(type_) }
	if range.min ~= nil or range.max ~= nil then
		filter.costRange = { min = range.min, max = range.max }
	end
	return {
		J.discover_from_library({
			step = "take",
			count = REVEAL_COUNT,
			player = "self",
			filter = filter,
			-- The prompt is a player's to read, and players read the rules' library as the Deck.
			prompt = "Reveal 3 cards from your deck; choose one to add to your hand",
		}),
	}
end

-- Step 4: "choose one to hand" — the revealed instance moves library → hand.
local function take_step(ctx: J.EffectContext): { J.EffectCall }
	return {
		J.add_to_hand({ instance = { of = "chosen" } }),
	}
end

local steps = {
	bracket = bracket_step,
	reveal = reveal_step,
	take = take_step,
}

local base = {
	cry = start_step,
	resume = steps,
}

local radiant = {
	cry = start_step,
	resume = steps,
	-- "Echo (resolves a second time)": one EXTRA resolution.
	staticFlags = { echo = 1 },
}

return {
	base = base,
	radiant = radiant,
}
