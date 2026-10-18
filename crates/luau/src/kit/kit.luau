--!strict
-- The card kit (#913, SPEC §12.3 to §12.5): the readable Luau a custom card is written in.
--
-- A custom card's script returns `Card { … }`. `Card` turns the author's table into the module shape
-- the host loads (`{ base = …, radiant = "base" }`): each hook becomes a function of the engine's
-- context that runs the author's function with `c`, the card's view of the moment, and returns the
-- effects `c`'s verbs recorded, built with `J`'s verbs, which the author never sees.
--
-- A question mid-script (`c:target(words)`, `c:choose(options)`) cannot suspend the script: the
-- engine keeps a paused game as plain data. So the run stops at the question with what it recorded,
-- and the answer runs the hook again from the top (the `kit` resume step) with every answer so far,
-- every board reading the earlier runs made (they answer as they did, so the run walks the same path)
-- and how many effects already resolved (they are not made again).
--
-- Every number crossing to the engine is an integer (L6), and nothing here iterates a table in the
-- hash's order (L7): the lint refuses this file as it refuses a card's.

local J = require("@jackioh")

-- The hooks a card may declare, and what each is.
local HOOKS = {
	cry = true,
	death = true,
	startOfTurn = true,
	endOfTurn = true,
}
local KEYS = "cry, death, startOfTurn, endOfTurn, aura, activate and target"

-- The resume step every question answers.
local STEP = "kit"

-- A run stops at a question by raising this table, which only `run` catches.
local PAUSE = {}

-- A word list: the words in the order every message names them, and what each names.
local function words(entries: { { any } }): any
	local list = { order = {}, of = {} }
	for _, entry in ipairs(entries) do
		table.insert(list.order, entry[1])
		list.of[entry[1]] = entry[2]
	end
	return list
end

-- Words that name one card or hero the player chooses (a `TargetScope`, and the declaration's filter).
local ONE = words({
	{ "an enemy unit", { side = "enemy", of = { "unit" } } },
	{ "an enemy", { side = "enemy", of = { "unit", "hero" } } },
	{ "an enemy unit or hero", { side = "enemy", of = { "unit", "hero" } } },
	{ "a friendly unit", { side = "ally", of = { "unit" }, excludeSelf = true } },
	{ "another friendly unit", { side = "ally", of = { "unit" }, excludeSelf = true } },
	{ "a friendly unit or hero", { side = "ally", of = { "unit", "hero" }, excludeSelf = true } },
	{ "a unit", { side = "any", of = { "unit" }, excludeSelf = true } },
	{ "another unit", { side = "any", of = { "unit" }, excludeSelf = true } },
	{ "a unit or hero", { side = "any", of = { "unit", "hero" }, excludeSelf = true } },
	{ "a character", { side = "any", of = { "unit", "hero" }, excludeSelf = true } },
})

-- Words that name one thing without a choice.
local FIXED = words({
	{ "your hero", { kind = "hero", side = "you" } },
	{ "the enemy hero", { kind = "hero", side = "enemy" } },
	{ "this unit", { kind = "self" } },
	{ "this card", { kind = "self" } },
})

-- Words that name every unit of a side, and the heroes too for the last ones.
local GROUPS = words({
	{ "each enemy unit", { side = "enemy", heroes = false } },
	{ "each friendly unit", { side = "self", heroes = false } },
	{ "each other friendly unit", { side = "self", heroes = false, excludeSelf = true } },
	{ "each unit", { side = "any", heroes = false } },
	{ "each other unit", { side = "any", heroes = false, excludeSelf = true } },
	{ "all enemies", { side = "enemy", heroes = true } },
	{ "each enemy", { side = "enemy", heroes = true } },
	{ "everything", { side = "any", heroes = true } },
})

-- Words that name one thing the match rng picks when the effect resolves.
local RANDOM = words({
	{ "a random enemy", { among = "enemies" } },
	{ "a random enemy unit", { among = "enemyUnits" } },
	{ "a random friendly unit", { friendly = true } },
})

-- The tokens `c:summon` knows, by name.
local TOKENS = words({
	{ "Rush Token", "core-t-rush" },
	{ "Sheep Token", "core-t-sheep" },
	{ "Felinor Token", "core-t-felinor" },
})

-- The keywords `c:give` and an aura's `give` may grant (`CUSTOM_KEYWORDS`).
local KEYWORDS = words({
	{ "Taunt", true },
	{ "Rush", true },
	{ "Charge", true },
	{ "First Strike", true },
	{ "Poisonous", true },
	{ "Lifesteal", true },
	{ "Reborn", true },
	{ "Divine Shield", true },
	{ "Trample", true },
	{ "Windfury", true },
	{ "Ranged", true },
	{ "Flying", true },
})

-- What `c:count` counts.
local COUNTS = words({
	{ "friendly units", { units = "you" } },
	{ "enemy units", { units = "enemy" } },
	{ "units", { units = "both" } },
	{ "cards in your hand", { hand = "you" } },
	{ "cards in the enemy hand", { hand = "enemy" } },
})

-- Where an aura's changes go.
local AURA_SIDES = words({
	{ "your units", "self" },
	{ "enemy units", "enemy" },
	{ "all units", "any" },
})

local function quoted(text: string): string
	return '"' .. text .. '"'
end

-- The words of some lists, quoted, for an error that names the choices.
local function names(...: any): string
	local out = {}
	for _, list in ipairs({ ... }) do
		for _, key in ipairs(list.order) do
			table.insert(out, quoted(key))
		end
	end
	return table.concat(out, ", ")
end

-- Raise `message` at the author's line: `depth` counts the kit's own calls between here and it.
local function fail(message: string, depth: number?): never
	error(message, (depth or 2) + 1)
end

-- A whole number from 0 up, or the error that says what `what` takes.
local function whole(value: any, what: string): number
	if type(value) == "function" then
		fail(what .. " takes a number: read the card's numbers as n.name, not c.name", 3)
	end
	if type(value) ~= "number" or value ~= math.floor(value) or value < 0 then
		fail(what .. " takes a whole number from 0 up, not " .. tostring(value), 3)
	end
	return value
end

---------------------------------------------------------------------------------------------------
-- c: the card's view of the moment
---------------------------------------------------------------------------------------------------

local C = {}

-- A board reading: the logged answer when an earlier run made this reading, else `read()` now,
-- logged for the runs after a question.
local function reading(c: any, read: () -> any): any
	c._read += 1
	local logged = c._reads[c._read]
	if logged ~= nil then
		return logged.v
	end
	local value = read()
	c._reads[c._read] = { v = value }
	return value
end

-- Record one effect, unless an earlier run already made it (it resolved before the question).
local function emit(c: any, effect: any)
	c._made += 1
	if c._made > c._skip then
		table.insert(c._effects, effect)
	end
end

local function me(c: any): string
	return c._ctx:controller()
end

local function them(c: any): string
	return c._ctx:opponent_of(me(c))
end

-- The engine's `TargetSpec` of a one-thing handle.
local function spec_of(c: any, handle: any): any
	if handle.kind == "self" then
		return { of = "self" }
	elseif handle.kind == "hero" then
		if handle.player == me(c) then
			return { of = "selfHero" }
		end
		return { of = "enemyHero" }
	end
	return { of = "instance", instanceId = handle.id }
end

local function self_id(c: any): string?
	local card = c._ctx:self_()
	if card == nil then
		return nil
	end
	return card.id
end

-- The ids of the units a group names, read off the board now (a logged reading).
local function group_ids(c: any, group: any): { string }
	return reading(c, function()
		local sides = {}
		if group.side == "self" or group.side == "any" then
			table.insert(sides, me(c))
		end
		if group.side == "enemy" or group.side == "any" then
			table.insert(sides, them(c))
		end
		local skip = if group.excludeSelf then self_id(c) else nil
		local ids = {}
		for _, player in ipairs(sides) do
			for _, unit in ipairs(c._ctx:active_units_of(player)) do
				if unit.id ~= skip then
					table.insert(ids, unit.id)
				end
			end
		end
		return ids
	end)
end

-- The heroes a group names, as one-thing handles.
local function group_heroes(c: any, group: any): { any }
	local heroes = {}
	if not group.heroes then
		return heroes
	end
	if group.side == "self" or group.side == "any" then
		table.insert(heroes, { kind = "hero", player = me(c) })
	end
	if group.side == "enemy" or group.side == "any" then
		table.insert(heroes, { kind = "hero", player = them(c) })
	end
	return heroes
end

-- A question: the answer an earlier run got, or a stop that asks it.
local function ask(c: any, question: any): any
	c._asked += 1
	local answer = c._answers[c._asked]
	if answer ~= nil then
		return answer
	end
	rawset(c, "_question", question)
	error(PAUSE, 0)
end

-- Whether a one-card word has anything to choose now (a logged reading).
local function anything_to_choose(c: any, scope: any): boolean
	return reading(c, function()
		for _, kind in ipairs(scope.of) do
			if kind == "hero" then
				return true
			end
		end
		local skip = if scope.excludeSelf then self_id(c) else nil
		local sides = {}
		if scope.side == "ally" or scope.side == "any" then
			table.insert(sides, me(c))
		end
		if scope.side == "enemy" or scope.side == "any" then
			table.insert(sides, them(c))
		end
		for _, player in ipairs(sides) do
			for _, unit in ipairs(c._ctx:active_units_of(player)) do
				if unit.id ~= skip then
					return true
				end
			end
		end
		return false
	end)
end

-- What a target argument names: a handle, or words, read as one of the lists above.
-- `{ kind = "none" }` is a choice with nothing to choose.
local function resolve(c: any, what: any, depth: number): any
	if type(what) == "table" then
		if what.kind == nil then
			fail("that is not a target: pass words like \"an enemy unit\" or what c:target answered", depth + 1)
		end
		return what
	end
	if what == nil then
		return { kind = "none" }
	end
	if type(what) ~= "string" then
		fail("a target is words like \"an enemy unit\", not " .. tostring(what), depth + 1)
	end
	local fixed = FIXED.of[what]
	if fixed ~= nil then
		if fixed.kind == "hero" then
			return { kind = "hero", player = if fixed.side == "you" then me(c) else them(c) }
		end
		return fixed
	end
	local group = GROUPS.of[what]
	if group ~= nil then
		return { kind = "group", side = group.side, heroes = group.heroes, excludeSelf = group.excludeSelf }
	end
	local random = RANDOM.of[what]
	if random ~= nil then
		return { kind = "random", among = random.among, friendly = random.friendly }
	end
	if ONE.of[what] ~= nil then
		return C.target(c, what)
	end
	fail(
		"no target is called "
			.. quoted(what)
			.. "; the words are "
			.. names(ONE, FIXED, GROUPS, RANDOM),
		depth + 1
	)
	return nil
end

-- `c:target(words)`: one card or hero the player chooses, asked now; with no words, the target the
-- card declared, chosen as it was played. Nil when there was nothing to choose.
function C.target(c: any, words: string?): any
	if words == nil then
		if not c._declared then
			fail("c:target() with no words reads the card's declared target, and this card declares none", 2)
		end
		-- The play's choice, read once and logged: a run after a question reads it as the first did,
		-- while the context's own choice is then the question's answer.
		local chosen = reading(c, function()
			local target = c._ctx:resolve_target({ of = "chosen" })
			if target == nil then
				return { kind = "none" }
			end
			if target.kind == "hero" then
				return { kind = "hero", player = target.player }
			end
			return { kind = "instance", id = target.instance.id }
		end)
		if chosen.kind == "none" then
			return nil
		end
		return chosen
	end
	local scope = ONE.of[words]
	if scope == nil then
		fail(
			"c:target asks for one card or hero: " .. quoted(words) .. " is not one of " .. names(ONE),
			2
		)
	end
	if not anything_to_choose(c, scope) then
		return nil
	end
	local answer = ask(c, { kind = "target", scope = scope, words = words })
	if answer.kind == "none" then
		return nil
	end
	return answer
end

-- `c:choose({ … })`: the option the player picks, by its text.
function C.choose(c: any, options: { string }): string
	if type(options) ~= "table" or #options < 2 then
		fail("c:choose takes a list of at least two options", 2)
	end
	for _, option in ipairs(options) do
		if type(option) ~= "string" then
			fail("each option of c:choose is text", 2)
		end
	end
	return ask(c, { kind = "mode", options = options })
end

-- Apply `one(spec)` to every one-thing a target names: a group's units and heroes, a choice, a fixed
-- handle. Nothing for a choice with nothing to choose.
local function each_one(c: any, target: any, one: (any) -> (), verb: string, depth: number)
	if target.kind == "none" then
		return
	elseif target.kind == "group" then
		for _, id in ipairs(group_ids(c, target)) do
			one({ of = "instance", instanceId = id })
		end
		for _, hero in ipairs(group_heroes(c, target)) do
			one(spec_of(c, hero))
		end
	elseif target.kind == "random" then
		fail(verb .. " cannot go to " .. "a random card; choose one, or name a group", depth + 1)
	else
		one(spec_of(c, target))
	end
end

function C.damage(c: any, what: any, amount: number)
	local n = whole(amount, "c:damage")
	local target = resolve(c, what, 2)
	if target.kind == "group" then
		emit(c, J.damage_all({ amount = n, side = target.side, heroes = target.heroes, excludeSelf = target.excludeSelf }))
	elseif target.kind == "random" then
		if target.friendly then
			fail("c:damage goes to \"a random enemy\" or \"a random enemy unit\"", 2)
		end
		emit(c, J.damage_split({ amount = n, among = target.among, perHit = n }))
	elseif target.kind ~= "none" then
		emit(c, J.damage({ to = spec_of(c, target), amount = n }))
	end
end

function C.heal(c: any, what: any, amount: number)
	local n = whole(amount, "c:heal")
	each_one(c, resolve(c, what, 2), function(spec)
		emit(c, J.heal({ target = spec, amount = n }))
	end, "c:heal", 2)
end

function C.destroy(c: any, what: any)
	each_one(c, resolve(c, what, 2), function(spec)
		emit(c, J.destroy({ target = spec }))
	end, "c:destroy", 2)
end

function C.bounce(c: any, what: any)
	each_one(c, resolve(c, what, 2), function(spec)
		emit(c, J.bounce({ target = spec }))
	end, "c:bounce", 2)
end

-- R1174000: Freeze the units it names; a hero in a group is passed by, as heroes are never Frozen.
function C.freeze(c: any, what: any)
	each_one(c, resolve(c, what, 2), function(spec)
		emit(c, J.freeze({ target = spec }))
	end, "c:freeze", 2)
end

function C.buff(c: any, what: any, attack: number, health: number?)
	local a = whole(attack, "c:buff")
	local h = whole(if health == nil then attack else health, "c:buff")
	local target = resolve(c, what, 2)
	if target.kind == "random" then
		if not target.friendly then
			fail("c:buff goes to \"a random friendly unit\", not a random enemy", 2)
		end
		emit(c, J.buff_random_unit({ player = "self", attack = a, health = h }))
		return
	end
	if target.kind == "group" and not target.heroes and not target.excludeSelf and target.side ~= "any" then
		emit(c, J.buff_all_units({ side = target.side, attack = a, health = h }))
		return
	end
	each_one(c, target, function(spec)
		if spec.of == "selfHero" or spec.of == "enemyHero" then
			return
		end
		emit(c, J.buff({ target = spec, attack = a, health = h }))
	end, "c:buff", 2)
end

function C.give(c: any, what: any, keyword: string)
	if type(keyword) ~= "string" or not KEYWORDS.of[keyword] then
		fail("c:give grants " .. names(KEYWORDS) .. ", not " .. tostring(keyword), 2)
	end
	each_one(c, resolve(c, what, 2), function(spec)
		if spec.of == "selfHero" or spec.of == "enemyHero" then
			return
		end
		emit(c, J.grant_keyword({ target = spec, keyword = { kind = keyword } }))
	end, "c:give", 2)
end

function C.draw(c: any, count: number?)
	local n = whole(if count == nil then 1 else count, "c:draw")
	if n > 0 then
		emit(c, J.draw({ count = n }))
	end
end

function C.discard(c: any, count: number?, who: string?)
	local n = whole(if count == nil then 1 else count, "c:discard")
	if who ~= nil and who ~= "you" and who ~= "the enemy" then
		fail("c:discard's third word is \"you\" or \"the enemy\"", 2)
	end
	if n > 0 then
		emit(c, J.discard_random({ count = n, player = if who == "the enemy" then "enemy" else "self" }))
	end
end

function C.summon(c: any, token: string, count: number?)
	local id = TOKENS.of[token]
	if id == nil then
		fail("c:summon summons " .. names(TOKENS) .. ", not " .. tostring(token), 2)
	end
	local n = whole(if count == nil then 1 else count, "c:summon")
	for _ = 1, n do
		emit(c, J.summon({ defId = id }))
	end
end

function C.armor(c: any, amount: number)
	local n = whole(amount, "c:armor")
	if n > 0 then
		emit(c, J.gain_hero_armor({ amount = n }))
	end
end

function C.mana(c: any, amount: number)
	local n = whole(amount, "c:mana")
	if n > 0 then
		emit(c, J.gain_mana({ amount = n }))
	end
end

-- `c:count(words)`: how many of something there are now.
function C.count(c: any, words: string): number
	local counted = COUNTS.of[words]
	if counted == nil then
		fail("c:count counts " .. names(COUNTS) .. ", not " .. tostring(words), 2)
	end
	return reading(c, function()
		if counted.hand ~= nil then
			return c._ctx:zone_count(if counted.hand == "you" then me(c) else them(c), "hand")
		end
		local n = 0
		if counted.units == "you" or counted.units == "both" then
			n += #c._ctx:active_units_of(me(c))
		end
		if counted.units == "enemy" or counted.units == "both" then
			n += #c._ctx:active_units_of(them(c))
		end
		return n
	end)
end

-- `c:health(target)`: a hero's or a unit's health now; 0 for nothing.
function C.health(c: any, what: any): number
	local target = resolve(c, what, 2)
	if target.kind == "group" or target.kind == "random" then
		fail("c:health reads one card or hero, not " .. tostring(what), 2)
	end
	return reading(c, function()
		if target.kind == "none" then
			return 0
		end
		if target.kind == "hero" then
			return c._ctx:hero_of(target.player).health
		end
		local id = if target.kind == "self" then self_id(c) else target.id
		if id == nil then
			return 0
		end
		local view = c._ctx:unit_view(id)
		if view == nil then
			return 0
		end
		return view.health
	end)
end

-- `c:mana_left()`: the controller's unspent mana now.
function C.mana_left(c: any): number
	return reading(c, function()
		return c._ctx:unspent_mana_of(me(c))
	end)
end

-- `c:each(words, function(card) … end)`: the statements for each unit (and hero) a group names.
function C.each(c: any, words: string, body: (any) -> ())
	local group = GROUPS.of[words]
	if group == nil then
		fail("c:each runs over " .. names(GROUPS) .. ", not " .. tostring(words), 2)
	end
	if type(body) ~= "function" then
		fail("c:each's second argument is a function of the card", 2)
	end
	local handle = { kind = "group", side = group.side, heroes = group.heroes, excludeSelf = group.excludeSelf }
	for _, id in ipairs(group_ids(c, handle)) do
		body({ kind = "instance", id = id })
	end
	for _, hero in ipairs(group_heroes(c, handle)) do
		body(hero)
	end
end

local C_META = {
	__index = function(c: any, key: string): any
		local method = C[key]
		if method ~= nil then
			return method
		end
		if key == "radiant" then
			return c._ctx:radiant()
		end
		if key == "your_turn" then
			return reading(c, function()
				return c._ctx:turn().active == me(c)
			end)
		end
		if key == "n" then
			return rawget(c, "_numbers")
		end
		fail("c has no " .. tostring(key) .. ": a card's numbers are n." .. tostring(key), 2)
		return nil
	end,
	__newindex = function(_c: any, key: any)
		fail("c is read-only: c." .. tostring(key) .. " cannot be set", 2)
	end,
}

-- Whether the card running declares the number `key` (a logged reading: `param` refuses any other).
local function declares(ctx: any, key: string): boolean
	local card = ctx:self_()
	if card == nil then
		return false
	end
	local def = ctx:def_of(card.defId)
	if def == nil or def.params == nil then
		return false
	end
	for _, param in ipairs(def.params) do
		if param.key == key then
			return true
		end
	end
	return false
end

-- `n`: the card's numbers by name, on the face running, read once a run and logged.
local function numbers(c: any): any
	return setmetatable({}, {
		__index = function(_n: any, key: any): any
			if type(key) ~= "string" then
				fail("a card's numbers have names: n." .. tostring(key) .. " is none", 2)
			end
			local value = reading(c, function()
				if not declares(c._ctx, key) then
					return nil
				end
				return c._ctx:param(key)
			end)
			if value == nil then
				fail("this card has no number called " .. key .. "; its text names its numbers as {" .. key .. "}", 2)
			end
			return value
		end,
		__newindex = function(_n: any, key: any)
			fail("a card's numbers are read-only: n." .. tostring(key) .. " cannot be set", 2)
		end,
	})
end

---------------------------------------------------------------------------------------------------
-- Runs
---------------------------------------------------------------------------------------------------

-- One run of the author's `body` with what earlier runs got (`past`, or nothing on the first run):
-- the effects it makes, and the question it stopped at as the last of them.
local function run(spec: any, hook: string, body: (any) -> (), ctx: any, past: any?): { any }
	local c: any = setmetatable({
		_ctx = ctx,
		_declared = spec.target ~= nil and (hook == "cry"),
		_answers = if past ~= nil then past.answers else {},
		_reads = if past ~= nil then past.reads else {},
		_skip = if past ~= nil then past.skip else 0,
		_read = 0,
		_asked = 0,
		_made = 0,
		_effects = {},
		_question = false,
		_numbers = false,
	}, C_META)
	rawset(c, "_numbers", numbers(c))
	local ok, stopped = pcall(body, c, c._numbers)
	if ok then
		return c._effects
	end
	if stopped ~= PAUSE then
		error(stopped, 0)
	end
	local question = c._question
	local data = {
		hook = hook,
		answers = c._answers,
		reads = c._reads,
		skip = c._made,
		question = question,
	}
	if question.kind == "target" then
		table.insert(c._effects, J.choose_target({
			step = STEP,
			scope = question.scope,
			prompt = "Choose " .. question.words,
			data = data,
		}))
	else
		table.insert(c._effects, J.choose_mode({
			step = STEP,
			options = question.options,
			prompt = "Choose one",
			data = data,
		}))
	end
	return c._effects
end

-- The answer a resumed question got, as a handle or an option.
local function answer_of(ctx: any, question: any): any
	if question.kind == "mode" then
		local picked = ctx:chosen_options()
		return picked[1]
	end
	local chosen = ctx:resolve_target({ of = "chosen" })
	if chosen == nil then
		return { kind = "none" }
	end
	if chosen.kind == "hero" then
		return { kind = "hero", player = chosen.player }
	end
	return { kind = "instance", id = chosen.instance.id }
end

---------------------------------------------------------------------------------------------------
-- Card { … }
---------------------------------------------------------------------------------------------------

local KEY_ORDER = { "cry", "death", "startOfTurn", "endOfTurn", "aura", "activate", "target" }

local function check_spec(spec: any)
	if type(spec) ~= "table" then
		fail("Card takes a table: Card { cry = function(c) … end }", 3)
	end
	-- Every key the author wrote must be one the kit knows; there is no `pairs` (L7), so the keys are
	-- counted instead: a table holding more keys than the known ones it holds has an unknown one.
	local known = 0
	for _, key in ipairs(KEY_ORDER) do
		if rawget(spec, key) ~= nil then
			known += 1
		end
	end
	local count = 0
	for _ in spec :: any do
		count += 1
	end
	if count ~= known then
		fail("Card knows " .. KEYS .. "; this card has another key", 3)
	end
	for _, hook in ipairs({ "cry", "death", "startOfTurn", "endOfTurn" }) do
		local body = rawget(spec, hook)
		if body ~= nil and type(body) ~= "function" then
			fail(hook .. " is a function of c: " .. hook .. " = function(c) … end", 3)
		end
	end
	if spec.target ~= nil and ONE.of[spec.target] == nil then
		fail("target is the words for one card or hero: " .. names(ONE), 3)
	end
	if spec.target ~= nil and spec.cry == nil then
		fail("a declared target is for the card's cry: give the card a cry", 3)
	end
	local aura = spec.aura
	if aura ~= nil then
		if type(aura) ~= "table" then
			fail("aura is a table: aura = { attack = 1, health = 1, to = \"your units\" }", 3)
		end
		if aura.to ~= nil and AURA_SIDES.of[aura.to] == nil then
			fail("an aura goes to " .. names(AURA_SIDES), 3)
		end
		if aura.give ~= nil then
			local given = if type(aura.give) == "table" then aura.give else { aura.give }
			for _, keyword in ipairs(given) do
				if not KEYWORDS.of[keyword] then
					fail("an aura gives the keywords c:give grants, not " .. tostring(keyword), 3)
				end
			end
		end
	end
	local activate = spec.activate
	if activate ~= nil then
		if type(activate) ~= "table" or type(activate.run) ~= "function" then
			fail("activate is a table: activate = { cost = 1, run = function(c) … end }", 3)
		end
		if activate.cost ~= nil then
			whole(activate.cost, "activate's cost")
		end
		if activate.label ~= nil and type(activate.label) ~= "string" then
			fail("activate's label is text", 3)
		end
	end
end

-- An aura's number: a whole number, or the name of one of the card's numbers (0 for a name the card
-- does not declare: an aura cannot fail mid-game, so `Card` cannot know the card's numbers to check).
local function aura_number(args: any, value: any): number?
	if value == nil then
		return nil
	end
	if type(value) == "string" then
		if not declares(args, value) then
			return 0
		end
		return args:param(value)
	end
	return value
end

local function Card(spec: any): any
	check_spec(spec)
	local face: any = {}
	local function hook_of(name: string, body: (any) -> ())
		return function(ctx: any)
			return run(spec, name, body, ctx, nil)
		end
	end
	for _, name in ipairs({ "cry", "death", "startOfTurn", "endOfTurn" }) do
		local body = rawget(spec, name)
		if body ~= nil and HOOKS[name] then
			face[name] = hook_of(name, body)
		end
	end
	if spec.target ~= nil then
		face.targets = { { kind = "target", min = 1, max = 1, filter = ONE.of[spec.target] } }
	end
	local aura = spec.aura
	if aura ~= nil then
		face.aura = function(args: any)
			local mod: any = {}
			local attack = aura_number(args, aura.attack)
			local health = aura_number(args, aura.health)
			if attack ~= nil then
				mod.attack = attack
			end
			if health ~= nil then
				mod.maxHealth = health
			end
			if aura.give ~= nil then
				mod.keywords = {}
				for _, keyword in ipairs(if type(aura.give) == "table" then aura.give else { aura.give }) do
					table.insert(mod.keywords, { kind = keyword })
				end
			end
			return { J.aura({ mod = mod, applies = { side = AURA_SIDES.of[aura.to or "your units"] } }) }
		end
	end
	local activate = spec.activate
	if activate ~= nil then
		local cost = if activate.cost ~= nil and activate.cost > 0 then { mana = activate.cost } else nil
		face.activations = {
			{
				id = "kit",
				label = activate.label or "Activate",
				uses = 1,
				cost = cost,
				run = function(ctx: any)
					return run(spec, "activate", activate.run, ctx, nil)
				end,
			},
		}
	end
	face.resume = {
		[STEP] = function(ctx: any)
			local data = ctx:data()
			local body = if data.hook == "activate" then activate.run else rawget(spec, data.hook)
			-- The step's data is the host's, read-only: the run gets copies it may add to.
			local answers = {}
			for _, answer in ipairs(data.answers or {}) do
				table.insert(answers, answer)
			end
			table.insert(answers, answer_of(ctx, data.question))
			local reads = {}
			for _, read in ipairs(data.reads or {}) do
				table.insert(reads, read)
			end
			return run(spec, data.hook, body, ctx, { answers = answers, reads = reads, skip = data.skip })
		end,
	}
	return { base = face, radiant = "base" }
end

return { Card = Card }
