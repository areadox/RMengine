-- Wird vor allen Skripten geladen: die Schnittstelle zur Engine.
-- Alles, was ein Skript "tut", landet in _RM.aktionen; die Engine führt es
-- nach dem Skript aus. So kann ein Skript die Welt nie halb kaputt machen.

_RM = {
  befehle = {}, betreten = {}, verlassen = {}, sieg = {}, takt = {}, sagen = {},
  timer = {}, aktionen = {}, fehler = {}, datei = "", jetzt = 0,
}

local function liste(w)
  if w == nil then return {} end
  if type(w) == "string" then return { w } end
  return w
end

local function tu(a) table.insert(_RM.aktionen, a) end

-- ---------------------------------------------------------------------
-- Anmelden: Befehle und Ereignisse
-- ---------------------------------------------------------------------

--- befehl{ woerter = {"kriech", "krieche"}, raum = "taverne", hilfe = "...", funktion = function(s, rest, raum) ... end }
function befehl(t)
  if type(t) ~= "table" then error("befehl{...} braucht eine Tabelle mit woerter und funktion", 2) end
  if type(t.funktion) ~= "function" then error("befehl: 'funktion' fehlt", 2) end
  local e = {
    woerter = liste(t.woerter or t.wort), raeume = liste(t.raeume or t.raum),
    gebiete = liste(t.gebiete or t.gebiet), hilfe = t.hilfe or "", syntax = t.syntax or "",
    funktion = t.funktion, datei = _RM.datei,
  }
  if #e.woerter == 0 then error("befehl: 'woerter' fehlt", 2) end
  table.insert(_RM.befehle, e)
end

local function ereignis(liste_name, wo, f, name)
  if type(f) ~= "function" then error(name .. ": Die Funktion fehlt", 3) end
  table.insert(_RM[liste_name], { wo = wo or "*", funktion = f, datei = _RM.datei })
end

--- bei_betreten("taverne", function(s, raum) ... end)   ("*" = jeder Raum)
function bei_betreten(raum, f) ereignis("betreten", raum, f, "bei_betreten") end
--- bei_verlassen("taverne", function(s, raum) ... end)
function bei_verlassen(raum, f) ereignis("verlassen", raum, f, "bei_verlassen") end
--- bei_sieg("wolf", function(s, monster_id, raum) ... end)
function bei_sieg(monster, f) ereignis("sieg", monster, f, "bei_sieg") end
--- bei_sagen("taverne", function(s, text, raum) ... end)
function bei_sagen(raum, f) ereignis("sagen", raum, f, "bei_sagen") end
--- bei_takt(function() ... end)   alle paar Sekunden (Takt der Welt)
function bei_takt(f) ereignis("takt", "*", f, "bei_takt") end

--- spaeter(3, function() ... end)   nach 3 Sekunden ausführen
function spaeter(sekunden, f)
  if type(f) ~= "function" then error("spaeter: Die Funktion fehlt", 2) end
  table.insert(_RM.timer, { zeit = _RM.jetzt + (tonumber(sekunden) or 0), funktion = f })
end

-- ---------------------------------------------------------------------
-- Der Spieler
-- ---------------------------------------------------------------------

Spieler = {}
Spieler.__index = Spieler

--- Text an den Spieler
function Spieler:sende(text) tu{ art = "sende", sid = self.sid, text = tostring(text) } end
--- Text an alle anderen im selben Raum
function Spieler:andere(text) tu{ art = "andere", sid = self.sid, text = tostring(text) } end
--- In einen anderen Raum bringen (weg/an = Texte für die anderen, optional)
function Spieler:gehe(raum, weg, an)
  tu{ art = "gehe", sid = self.sid, raum = tostring(raum), weg = weg, an = an }
  self.raum = tostring(raum)
end
--- Hat er den Gegenstand dabei (oder angelegt)?
function Spieler:hat(id)
  for _, g in ipairs(self.inventar) do if g == id then return true end end
  for _, g in pairs(self.angelegt) do if g == id then return true end end
  return false
end
--- Einen Gegenstand geben (anzahl optional)
function Spieler:gib(id, anzahl)
  for _ = 1, (anzahl or 1) do
    tu{ art = "gib", sid = self.sid, id = tostring(id) }
    table.insert(self.inventar, tostring(id))
  end
end
--- Einen Gegenstand wegnehmen; false, wenn er ihn nicht hat
function Spieler:nimm(id)
  for i, g in ipairs(self.inventar) do
    if g == id then
      table.remove(self.inventar, i)
      tu{ art = "nimm", sid = self.sid, id = id }
      return true
    end
  end
  return false
end
--- Geld geben (negativ = abziehen)
function Spieler:geld_aendern(n)
  n = math.tointeger(n) or math.floor(n)
  tu{ art = "geld", sid = self.sid, n = n }
  self.geld = math.max(0, self.geld + n)
end
--- Bezahlen: false, wenn das Geld nicht reicht
function Spieler:zahle(n)
  if self.geld < n then return false end
  self:geld_aendern(-n)
  return true
end
function Spieler:heile(n) tu{ art = "lp", sid = self.sid, n = math.floor(n) }; self.lp = math.min(self.max_lp, self.lp + n) end
function Spieler:schade(n) tu{ art = "lp", sid = self.sid, n = -math.floor(n) }; self.lp = self.lp - n end
function Spieler:mana(n) tu{ art = "mp", sid = self.sid, n = math.floor(n) } end
function Spieler:erfahrung(n) tu{ art = "erfahrung", sid = self.sid, n = math.floor(n) } end
--- Sich etwas dauerhaft merken (bleibt gespeichert): s:merke("muenze", true)
function Spieler:merke(name, wert)
  tu{ art = "merke", sid = self.sid, name = tostring(name), wert = wert }
  self.merker[tostring(name)] = wert
end
--- Einen ganz normalen Befehl ausführen, z.B. s:befehl("schau")
function Spieler:befehl(text) tu{ art = "befehl", sid = self.sid, text = tostring(text) } end
--- Fortschritt in einer Quest mit einem Schritt art = "skript", ziel = ...
function Spieler:fortschritt(ziel) tu{ art = "fortschritt", sid = self.sid, ziel = tostring(ziel) } end
--- Die Wirkungen eines Zaubers auf ihn legen (z.B. einen Schild), ohne Kosten
function Spieler:zauber(id) tu{ art = "zauber", sid = self.sid, id = tostring(id) } end

-- ---------------------------------------------------------------------
-- Die Welt
-- ---------------------------------------------------------------------

welt = { merker = {} }

--- Text an alle in einem Raum
function welt.sende_raum(raum, text) tu{ art = "raum", raum = tostring(raum), text = tostring(text) } end
--- Text an alle Spieler
function welt.sende_alle(text) tu{ art = "alle", text = tostring(text) } end
--- Einen Gegenstand in einen Raum legen / daraus entfernen
function welt.lege(raum, id) tu{ art = "lege", raum = tostring(raum), id = tostring(id) } end
function welt.entferne(raum, id) tu{ art = "entferne", raum = tostring(raum), id = tostring(id) } end
--- Ein Monster oder einen NPC in einem Raum erscheinen lassen
function welt.erschaffe(raum, monster) tu{ art = "erschaffe", raum = tostring(raum), id = tostring(monster) } end
--- Sich etwas für die ganze Welt merken (bleibt gespeichert)
function welt.merke(name, wert)
  tu{ art = "welt_merke", name = tostring(name), wert = wert }
  welt.merker[tostring(name)] = wert
end

-- ---------------------------------------------------------------------
-- Für die Engine
-- ---------------------------------------------------------------------

local function sicher(e, ...)
  local ok, fehler = pcall(e.funktion, ...)
  if not ok then table.insert(_RM.fehler, (e.datei or "?") .. ": " .. tostring(fehler)) end
  return ok, fehler
end

function _RM.befehl(i, s, rest, raum)
  local e = _RM.befehle[i]
  local ok, ergebnis = pcall(e.funktion, setmetatable(s, Spieler), rest, raum)
  if not ok then
    table.insert(_RM.fehler, e.datei .. ": " .. tostring(ergebnis))
    return true
  end
  return ergebnis ~= false
end

function _RM.ereignis(art, wo, s, a, b)
  if s then setmetatable(s, Spieler) end
  for _, e in ipairs(_RM[art]) do
    if e.wo == "*" or e.wo == wo then sicher(e, s, a, b) end
  end
end

function _RM.timer_pruefen()
  local faellig, rest = {}, {}
  for _, t in ipairs(_RM.timer) do
    if t.zeit <= _RM.jetzt then table.insert(faellig, t) else table.insert(rest, t) end
  end
  _RM.timer = rest
  for _, t in ipairs(faellig) do sicher(t) end
  local naechster = nil
  for _, t in ipairs(_RM.timer) do
    if naechster == nil or t.zeit < naechster then naechster = t.zeit end
  end
  return naechster
end
