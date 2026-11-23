--!strict
-- Spec for `luau/src/Host.luau` against `luau/tests/mock.luau`: a tiny assert
-- helper, no framework. Exits non-zero on the first failure after printing
-- every check's line (`@lune/process`).

local process = require("@lune/process")
local Host = require("../src/Host")
local Mock = require("./mock")

local failures = 0

local function check(name: string, cond: boolean): ()
	if cond then
		print("ok " .. name)
	else
		failures += 1
		print("FAIL " .. name)
	end
end

local function fresh(): any
	local mock = Mock.new()
	return { mock = mock, host = Host.new(mock:instance()) }
end

local function checkLive(mock: any, tag: string): ()
	check("no live allocations (" .. tag .. ")", mock:liveCount() == 0)
end

do
	local f = fresh()
	f.host:call("echo", "hello")
	f.host:call("echo", "world")
	f.host:call("echo", "!")
	check("init once", f.mock.initCalls == 1)
	checkLive(f.mock, "init once")
end

do
	local f = fresh()
	check("ascii round trip", f.host:call("echo", "hello") == "hello")
	checkLive(f.mock, "ascii")
end

do
	local f = fresh()
	local s = "héllo ★"
	local back = f.host:call("echo", s)
	check("utf8 round trip", back == s and #s == 10)
	checkLive(f.mock, "utf8")
end

do
	local f = fresh()
	check("empty string", f.host:call("echo", "") == "" and f.mock.allocCalls == 0)
	checkLive(f.mock, "empty string")
end

do
	local f = fresh()
	check("empty result", f.host:call("empty") == "" and f.mock.ptrCalls == 0)
	checkLive(f.mock, "empty result")
end

do
	local f = fresh()
	check("two strings in order", f.host:call("concat", "ab", "cd") == "abcd")
	checkLive(f.mock, "concat")
end

do
	local f = fresh()
	check("number", f.host:call("addone", 41) == "42")
	checkLive(f.mock, "number")
end

do
	local f = fresh()
	check("zero args", f.host:call("ping") == "pong")
	checkLive(f.mock, "zero args")
end

do
	local f = fresh()
	local ok, err = pcall(f.host.call, f.host, "refuse", "x")
	check("status 1", not ok and err == "refuse refused: nope: bad deck")
	checkLive(f.mock, "status 1")
	check("host fine after refusal", f.host:call("echo", "again") == "again")
	checkLive(f.mock, "after refusal")
end

do
	local f = fresh()
	check("panic setup", f.host:call("echo", "prev") == "prev")
	local echoCalls = f.mock.bindingCalls.echo
	local ok, err = pcall(f.host.call, f.host, "panic")
	check("panic trap", not ok and err == "panic trapped: panicked at 'boom', src/lib.rs:1:1")
	checkLive(f.mock, "panic trap")
	local ok2, err2 = pcall(f.host.call, f.host, "echo", "x")
	check(
		"dead refuses fresh instance",
		not ok2 and err2 == "echo refused: instance is dead; open a fresh one" and f.mock.bindingCalls.echo == echoCalls
	)
	checkLive(f.mock, "after panic")
end

do
	local f = fresh()
	check("silent setup", f.host:call("echo", "prev") == "prev")
	local ok, err = pcall(f.host.call, f.host, "silent")
	check("silent trap", not ok and err == "silent trapped: no text")
	checkLive(f.mock, "silent trap")
end

do
	local f = fresh()
	f.mock:grow()
	check("grow between calls", f.host:call("echo", "hello") == "hello")
	checkLive(f.mock, "grow between")
end

do
	local f = fresh()
	f.mock.growOnAlloc = true
	check("grow inside alloc", f.host:call("echo", "hello") == "hello")
	checkLive(f.mock, "grow in alloc")
end

do
	local f = fresh()
	f.mock.growOnCall = true
	check("grow inside binding", f.host:call("echo", "hello") == "hello")
	checkLive(f.mock, "grow in binding")
end

do
	local answer = [[{"state":{"a":{"b":1},"s":"x\"y"},"events":[]}]]
	check("splitState nested braces", Host.splitState(answer) == [[{"a":{"b":1},"s":"x\"y"}]])
end

do
	local answer = [[{"events":[],"state":{"x":[1,{"y":"z"}]}}]]
	check("splitState state second", Host.splitState(answer) == [[{"x":[1,{"y":"z"}]}]])
end

do
	local mock = Mock.new()
	local ok1 = pcall(mock.exports.jackioh_free, 999, 1)
	local ptr = mock.exports.jackioh_alloc(4)
	local ok2 = pcall(mock.exports.jackioh_free, ptr, 5)
	check("bad free raises", not ok1 and not ok2)
end

do
	local f = fresh()
	local ok, err = pcall(f.host.call, f.host, "nope")
	check("unknown binding", not ok and err == "nope refused: unknown binding")
	check("host fine after unknown", f.host:call("echo", "still here") == "still here")
	checkLive(f.mock, "unknown binding")
end

if failures > 0 then
	print(failures .. " failing check(s)")
	process.exit(1)
end
print("all checks passed")
