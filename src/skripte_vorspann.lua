-- Wird vor allen Skripten geladen: die Schnittstelle zur Engine.
-- Alles, was ein Skript "tut", landet in _RM.aktionen; die Engine führt es
-- nach dem Skript aus. So kann ein Skript die Welt nie halb kaputt machen.

_RM = {
  befehle = {}, ereignisse = {}, timer = {}, aktionen = {}, fehler = {},
  datei = "", jetzt = 0,
  -- Bei Skripten an Räumen, Gegenständen und NPCs: wem das Skript gehört
  hier = nil,
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
--- Weitere Grenzen: gebiet = "...", gegenstand = "..." (man hat ihn dabei oder er liegt im Raum),
--- npc = "..." (der NPC/das Monster ist im Raum). Im Skript eines Raums/Gegenstands/NPCs gilt der Befehl dort.
function befehl(t)
  if type(t) ~= "table" then error("befehl{...} braucht eine Tabelle mit woerter und funktion", 2) end
  if type(t.funktion) ~= "function" then error("befehl: 'funktion' fehlt", 2) end
  local e = {
    woerter = liste(t.woerter or t.wort), raeume = liste(t.raeume or t.raum),
    gebiete = liste(t.gebiete or t.gebiet), gegenstaende = liste(t.gegenstaende or t.gegenstand),
    npcs = liste(t.npcs or t.npc), hilfe = t.hilfe or "", syntax = t.syntax or "",
    funktion = t.funktion, datei = _RM.datei,
  }
  if #e.woerter == 0 then error("befehl: 'woerter' fehlt", 2) end
  local h = _RM.hier
  if h and #e.raeume == 0 and #e.gebiete == 0 and #e.gegenstaende == 0 and #e.npcs == 0 then
    if h.art == "raum" then e.raeume = { h.id }
    elseif h.art == "gegenstand" then e.gegenstaende = { h.id }
    else e.npcs = { h.id } end
  end
  table.insert(_RM.befehle, e)
end

-- Ein Ereignis anmelden. Erster Wert = wo (Raum, Gegenstand oder Monster/NPC);
-- fehlt er, gilt im Skript eines Raums/Gegenstands/NPCs dieser, sonst "*" (überall).
local function anmelden(art, wo, f)
  if type(wo) == "function" and f == nil then
    f = wo
    wo = _RM.hier and _RM.hier.id or "*"
  end
  if type(f) ~= "function" then error("bei_" .. art .. ": Die Funktion fehlt", 3) end
  _RM.ereignisse[art] = _RM.ereignisse[art] or {}
  table.insert(_RM.ereignisse[art], { wo = wo or "*", funktion = f, datei = _RM.datei })
end

-- Räume: (s, ..., raum)
function bei_betreten(wo, f) anmelden("betreten", wo, f) end   -- (s, raum)
function bei_verlassen(wo, f) anmelden("verlassen", wo, f) end -- (s, raum)
function bei_sagen(wo, f) anmelden("sagen", wo, f) end         -- (s, text, raum)
function bei_gehen(wo, f) anmelden("gehen", wo, f) end         -- (s, richtung, raum)  false = verhindern
function bei_tuer(wo, f) anmelden("tuer", wo, f) end           -- (s, aktion, richtung, raum)  false = verhindern
function bei_ansehen(wo, f) anmelden("ansehen", wo, f) end     -- (s, was, raum)  false = normale Anzeige weglassen
-- Gegenstände: (s, id, raum), false = verhindern
function bei_nehmen(wo, f) anmelden("nehmen", wo, f) end
function bei_ablegen(wo, f) anmelden("ablegen", wo, f) end
function bei_benutzen(wo, f) anmelden("benutzen", wo, f) end
function bei_anlegen(wo, f) anmelden("anlegen", wo, f) end
function bei_ausziehen(wo, f) anmelden("ausziehen", wo, f) end
-- NPCs und Monster: (s, ..., wesen, raum)
function bei_gespraech(wo, f) anmelden("gespraech", wo, f) end -- (s, text, npc, raum)  false = keine normale Antwort
function bei_bekommen(wo, f) anmelden("bekommen", wo, f) end   -- (s, id, npc, raum)  false = nimmt es nicht an
function bei_angriff(wo, f) anmelden("angriff", wo, f) end     -- (s, monster, raum)  false = verhindern
function bei_sieg(wo, f) anmelden("sieg", wo, f) end           -- (s, monster_id, raum)
-- Sonst
function bei_takt(f) anmelden("takt", "*", f) end              -- alle paar Sekunden

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

--- Ein Ausgang: welt.ausgang("keller", "nord", "geheimgang"), sperren, wieder normal
function welt.ausgang(raum, richtung, ziel) tu{ art = "ausgang", raum = tostring(raum), richtung = tostring(richtung), ziel = tostring(ziel) } end
function welt.ausgang_sperren(raum, richtung) tu{ art = "ausgang", raum = tostring(raum), richtung = tostring(richtung), sperren = true } end
function welt.ausgang_normal(raum, richtung) tu{ art = "ausgang", raum = tostring(raum), richtung = tostring(richtung), normal = true } end
--- Tür: "offen", "zu" oder "verschlossen"
function welt.tuer(raum, richtung, zustand) tu{ art = "tuer", raum = tostring(raum), richtung = tostring(richtung), zustand = tostring(zustand) } end
--- Zusatzzeile unter der Raumbeschreibung (nil = wieder weg)
function welt.raum_zusatz(raum, text) tu{ art = "zusatz", raum = tostring(raum), text = text and tostring(text) or nil } end

-- ---------------------------------------------------------------------
-- NPCs und Monster ("Wesen")
-- ---------------------------------------------------------------------

Wesen = {}
Wesen.__index = Wesen

--- Der NPC sagt etwas: „...“
function Wesen:sage(text) tu{ art = "wesen_sagt", uid = self.uid, text = tostring(text) } end
--- Der NPC tut etwas: "Der Wirt nickt."
function Wesen:tu(text) tu{ art = "wesen_tut", uid = self.uid, text = tostring(text) } end
function Wesen:schade(n) tu{ art = "wesen_lp", uid = self.uid, n = -math.floor(n) } end
function Wesen:heile(n) tu{ art = "wesen_lp", uid = self.uid, n = math.floor(n) } end
--- In einen anderen Raum schicken
function Wesen:gehe(raum) tu{ art = "wesen_gehe", uid = self.uid, raum = tostring(raum) } end
--- Verschwinden lassen (kommt beim Auffüllen der Welt wieder)
function Wesen:verschwinde() tu{ art = "wesen_weg", uid = self.uid } end
--- Einem Spieler etwas geben
function Wesen:gib(s, id) s:gib(id) end

-- ---------------------------------------------------------------------
-- ---------------------------------------------------------------------
-- Für die Engine
-- ---------------------------------------------------------------------

local function sicher(e, ...)
  local ok, ergebnis = pcall(e.funktion, ...)
  if not ok then table.insert(_RM.fehler, (e.datei or "?") .. ": " .. tostring(ergebnis)) end
  return ok, ergebnis
end

function _RM.befehl(i, s, rest, raum)
  local e = _RM.befehle[i]
  for _, w in ipairs(raum.wesen or {}) do setmetatable(w, Wesen) end
  local ok, ergebnis = pcall(e.funktion, setmetatable(s, Spieler), rest, raum)
  if not ok then
    table.insert(_RM.fehler, e.datei .. ": " .. tostring(ergebnis))
    return true
  end
  return ergebnis ~= false
end

-- Ein Ereignis an alle passenden Funktionen; gibt false zurück, wenn eine
-- davon false zurückgibt (= verhindern)
function _RM.ereignis(art, wo, s, raum, ...)
  if s then setmetatable(s, Spieler) end
  local erlaubt = true
  local mitte = table.pack(...)
  for i = 1, mitte.n do
    if type(mitte[i]) == "table" and mitte[i].uid then setmetatable(mitte[i], Wesen) end
  end
  for _, e in ipairs(_RM.ereignisse[art] or {}) do
    if e.wo == "*" or e.wo == wo then
      local args = {}
      for i = 1, mitte.n do args[i] = mitte[i] end
      args[mitte.n + 1] = raum
      local ok, ergebnis = sicher(e, s, table.unpack(args, 1, mitte.n + 1))
      if ok and ergebnis == false then erlaubt = false end
    end
  end
  return erlaubt
end

--- Gibt es Funktionen für dieses Ereignis (an diesem Ort)?
function _RM.hat(art, wo)
  for _, e in ipairs(_RM.ereignisse[art] or {}) do
    if e.wo == "*" or e.wo == wo then return true end
  end
  return false
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
