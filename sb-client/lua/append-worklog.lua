-- Dopisuje jeden wpis worklog do strony miesięcznej.
-- Wywołanie:
--   sbcall -f append_worklog.lua \
--     -a started_at="2026-08-18 10:09:36" -a ended_at="2026-08-18 12:23:37" \
--     -a task="axi-slave zebu rework" -a tag="work" \
--     -a minimum_min=30 -a worked_min=89 -a paused_min=45

local function required(key)
  local v = ARGS[key]
  if not v or v == "" then
    error("brakuje wymaganego argumentu: " .. key)
  end
  return v
end

local started_at = required("started_at")
local ended_at   = required("ended_at")
local task       = required("task")
local tag        = ARGS.tag or "work"

-- strona miesięczna: Worklog/2026-08
local month = string.sub(started_at, 1, 7)
local page = "Worklog/" .. month

-- pozycja listy z atrybutami — indeksuje się jako obiekt z tagiem #worklog
local line = string.format(
  "- %s → %s %s #%s [started_at: %s] [ended_at: %s] [minimum_min: %s] [worked_min: %s] [paused_min: %s]",
  string.sub(started_at, 12, 16),
  string.sub(ended_at, 12, 16),
  task,
  tag,
  started_at,
  ended_at,
  ARGS.minimum_min or "0",
  ARGS.worked_min or "0",
  ARGS.paused_min or "0"
)

local ok, existing = pcall(space.readPage, page)
if not ok or not existing or existing == "" then
  existing = "---\ntags: meta\n---\n\n# Worklog " .. month .. "\n"
end

if not string.endsWith(existing, "\n") then
  existing = existing .. "\n"
end

space.writePage(page, existing .. line .. "\n")
return { page = page, line = line }
