#!/usr/bin/env bash
# c.sh - thin curl wrapper for the CDP server (the Crumbtrail dev window).
# Start: `npm run cdp:dev` (de-elevated app + CDP :9222), then `npm run cdp:serve` (this API, :9223).
#
#   bash scripts/cdp/c.sh look                  # VERIFY PRIMITIVE: state+errors+shot in ONE call; Read the path on the LAST line
#   bash scripts/cdp/c.sh look ".panel"         # same, shot clipped to a selector
#   bash scripts/cdp/c.sh peek                  # look WITHOUT the shot (state+errors, 0 image tokens)
#   bash scripts/cdp/c.sh state                 # exact app state from window.__crumb: tab/view/popover/sheet/scans/errors
#
# NAVIGATE — real clicks by visible label, prerequisites included, then settle + look:
#   bash scripts/cdp/c.sh nav space             # clean | space | changed | folders | files | browse | games | installers | types
#   bash scripts/cdp/c.sh nav games             #   = Space tab -> More ▾ -> Games (views need a scan: c.sh scan C:)
#   bash scripts/cdp/c.sh nav review            # opens the review sheet ("Clean 15 GB?"); `nav esc` closes it. NEVER confirms.
#   bash scripts/cdp/c.sh nav welcome|theme|esc
#   bash scripts/cdp/c.sh nav "Rescan"          # anything else = click that visible label
#   bash scripts/cdp/c.sh scan C:               # Space tab -> drive card -> wait for the scan to finish -> look
#   bash scripts/cdp/c.sh tour clean space changed games   # visit N surfaces + shot EACH in ONE round-trip
#
# SELECTORS (click/act/measure/...): plain CSS, or by label — survives restyles and state classes:
#   text=Rescan   tab=Biggest folders   menuitem=Games   button=Toggle theme   "button.drive >> C:"
#
# INSPECT:  find "Save" · ax [sel] · text ".panel" · errors [--all] · console · measure ".hero" · shot-sel ".sheet" jpeg 70 hover
# ACT:      act click 'tab=Space' (click+quiesce+look) · act key Escape · type "input" "text" Enter · click "<sel>" · key Escape
# WAIT/RUN: wait "<js>" 30000 · eval "<js>" · ready (mounted + idle) · reload · batch '<json>'
# COMPARE:  baseline / diff (before-after pixels) · shot [png 0] · shot-sel "<sel>"
# HEALTH:   health · doctor (why is CDP down) · reap [--all] (orphaned dev procs) · reset-viewport · shutdown
#
# SAFETY: this drives the REAL app on the real PC. `click` refuses "Clean now", "Retry skipped" and
# "Restart as admin" (a real clean / a UAC relaunch that kills CDP) unless serve.cjs runs with CRUMB_ALLOW_CLEAN=1.
set -euo pipefail
API="${RIFT_CDP_API:-http://127.0.0.1:9223}"
TARGET="${RIFT_CDP_TARGET:-}"
if [ "${1:-}" = "-t" ]; then TARGET="${2:-}"; shift 2; fi
cmd="${1:-}"; shift || true
# Target is carried as a query param (server reads query before body), so it
# applies uniformly to GET and POST without touching each JSON body.
qs=""; [ -n "$TARGET" ] && qs="?target=$TARGET"

# JSON encoding is jq, not a per-call `node -e` spawn (~37ms vs ~76ms cold).
# jq --arg/--argjson handle arbitrary quotes/newlines in JS expressions safely.
command -v jq >/dev/null 2>&1 || { echo "c.sh requires jq (winget install jqlang.jq)" >&2; exit 3; }

# GET/POST helpers. We deliberately DON'T use `curl -f`: on an HTTP error `-f`
# discards the response body and prints only "curl: (22) ... 500", swallowing the
# server's structured `{error}` message (the #1 "tool silently failed" cause). The
# server now returns expected errors as 200; for a genuine 500 we still want the
# JSON body, so we capture it and let the caller's jq surface `.error`. A real
# transport failure (server down) yields empty output + a clear stderr note.
http_get() {
  local out; out="$(curl -sS "$1" 2>/dev/null)" || true
  if [ -z "$out" ]; then echo "c.sh: no response from $API (is 'npm run cdp:serve' running?)" >&2; return 7; fi
  printf '%s' "$out"
}
post() {
  local out; out="$(curl -sS -X POST "$API/$1$qs" -H 'Content-Type: application/json' --data "$2" 2>/dev/null)" || true
  if [ -z "$out" ]; then echo "c.sh: no response from $API/$1 (is 'npm run cdp:serve' running?)" >&2; return 7; fi
  printf '%s' "$out"
}

# Shared jq renderer for a /look payload (also the last op of act/nav batches).
# Honest by construction: app-dead, dom-scrape downgrade, stale-error counts,
# viewport-suspect and per-tab errors all SURFACE — nothing silently hides.
LOOK_JQ='def looksum(l):
  (l.page // {}) as $p |
  if ($p.error) then
    ("[look] ✗ app unreachable: " + ($p.error|tostring) + " — run: bash scripts/cdp/c.sh doctor")
  else (
    "[look] " + ($p.tab // (($p.tabs // []) | join("+")) // "?")
      + (if $p.view then "/" + ($p.view|tostring) else "" end)
      + (if ($p.crumbs // []) | length > 0 then " (" + ($p.crumbs | join(" › ")) + ")" else "" end)
      + (if $p.popover then " · popover=" + ($p.popover|tostring) else "" end)
      + (if $p.sheetOpen or $p.dialog then " · REVIEW SHEET OPEN" else "" end)
      + (if $p.welcomeOpen then " · WELCOME CARD OPEN" else "" end)
      + (if $p.scanning then " · scanning" else "" end)
      + (if $p.cleaning then " · CLEANING " + ($p.cleanStep|tostring) else "" end)
      + (if $p.spaceScanning then " · space-scanning " + ($p.spaceRoot|tostring) else "" end)
      + (if $p.diffLoading then " · diff-loading" else "" end)
      + (if ($p.update // "idle") != "idle" then " · update=" + ($p.update|tostring) else "" end)
      + (if $p.source == "dom" then " · (dom-scrape fallback: no window.__crumb)" else "" end)
      + " · " + ($p.theme // "?") + (if $p.admin == false then " · non-admin" else "" end)
      + (if $p.selected != null then " · selected=" + ($p.selected|tostring) else "" end)
      + (if $p.vp then " · vp=" + ($p.vp.w|tostring) + "x" + ($p.vp.h|tostring) else "" end),
    ( [ ["app-error",$p.appError], ["space-error",$p.spaceError], ["diff-error",$p.diffError], ["scan-error",$p.scanError], ["preset-note",$p.presetNote] ]
      | .[] | select(.[1]) | "[" + .[0] + "] " + (.[1]|tostring|.[0:200]) ),
    "[errors] " + ((l.errorCount // 0)|tostring)
      + (if (l.staleErrors // 0) > 0 then " (+" + (l.staleErrors|tostring) + " stale hidden — c.sh errors --all)" else "" end),
    (l.errors[]? | "  ✗ " + (.text // "?")),
    (if l.viewportSuspect then "⚠ viewport-suspect — a capture failed to clear its size override; run: bash scripts/cdp/c.sh reset-viewport" else empty end),
    (if l.shot then (l.shot.path // ("(shot failed: " + (l.shot.error // "?") + ")")) else empty end)
  ) end;
'
# Renderer for an action result (click/key op) — surfaces errors + selector
# suggestions + covered-click warnings that used to be silently swallowed.
ACT_JQ='def actsum(a; tag):
  if (a.error) then
    ("[" + tag + "] ✗ " + (a.error|tostring)
      + (if (a.suggestions // []) | length > 0 then
          "\n  did you mean:" + ([a.suggestions[] | "\n    " + .selector + "   ← " + ((.text // "")|.[0:40]) + (if .visible then "" else " [hidden]" end)] | join(""))
        else "" end))
  else
    ("[" + tag + "] ✓"
      + (if a.via then " via=" + a.via else "" end)
      + (if a.reason then " (" + a.reason + ")" else "" end)
      + (if a.covered then "  ⚠ COVERED by " + ((a.coveredBy // "?")|tostring) + " — the click may have landed on an overlay" else "" end))
  end;
def settlesum(s):
  if (s.error) then ("[settled] ✗ " + (s.error|tostring))
  elif (s.quiet == false) then ("[settled] " + ((s.waitedMs // 0)|tostring) + "ms CAPPED — DOM mutating, or " + ((s.animating // 0)|tostring) + " animation(s) still running (shot may be mid-transition)")
  elif (s.waitedMs != null) then ("[settled] " + (s.waitedMs|tostring) + "ms quiet, " + ((s.mutations // 0)|tostring) + " mutations")
  else ("[settled] " + ((s.sleptMs // 0)|tostring) + "ms (fixed)")
  end;
'

# Crumbtrail destinations -> JSON array of click ops (shared by nav + tour). Real clicks by visible label
# (`tab=` / `menuitem=` / `button=` / `text=`, see resolveEl in serve.cjs). Every destination clicks its
# own prerequisites, so it works from anywhere: `games` = Space tab -> More ▾ -> Games. Safe to repeat:
# setTab/setView close any open popover first. Popover trigger is matched by aria-haspopup because the
# More button's text changes to the active view's name ("Games ▾").
nav_ops() {
  local -a steps=()
  case "$1" in
    clean)         steps=("tab=Clean") ;;
    space)         steps=("tab=Space") ;;
    changed)       steps=("tab=Space" "tab=What changed") ;;
    folders|hot)   steps=("tab=Space" "tab=Biggest folders") ;;
    files|big)     steps=("tab=Space" "tab=Largest files") ;;
    browse)        steps=("tab=Space" "tab=Browse") ;;
    games)         steps=("tab=Space" '[aria-haspopup="menu"]' "menuitem=Games") ;;
    installers)    steps=("tab=Space" '[aria-haspopup="menu"]' "menuitem=Forgotten installers") ;;
    types)         steps=("tab=Space" '[aria-haspopup="menu"]' "menuitem=File types") ;;
    review)        steps=("tab=Clean" "button.go") ;;
    welcome|help)  steps=("button=?") ;;
    theme)         steps=("button=Toggle theme") ;;
    esc)           ;;
    *)             steps=("text=$1") ;;
  esac
  if [ "$1" = esc ]; then printf '%s' '[{"op":"key","params":{"key":"Escape"}}]'; return; fi
  local ops='[]' i=0 s
  for s in "${steps[@]}"; do
    [ "$i" -gt 0 ] && ops="$(jq -c '. + [{op:"settle",params:{maxMs:200,quietMs:80}}]' <<<"$ops")"
    ops="$(jq -c --arg s "$s" '. + [{op:"click",params:{selector:$s}}]' <<<"$ops")"
    i=$((i + 1))
  done
  printf '%s' "$ops"
}
# Space views only exist once a scan is loaded — nav uses this to explain a miss.
nav_needs_scan() {
  case "$1" in changed|folders|hot|files|big|browse|games|installers|types) echo 1 ;; *) echo 0 ;; esac
}


case "$cmd" in
  health|state|page|targets)
    http_get "$API/$cmd$qs"
    ;;
  ax)
    # ax [selector] [full] [limit] — image-FREE structural snapshot via the a11y
    # tree. Answers "what's on screen + what can I click" for ~0 image tokens.
    #   c.sh ax                  -> controls + landmarks + headings, whole page
    #   c.sh ax ".ah-wrap"       -> scope to a selector's subtree
    #   c.sh ax "" full          -> every named non-ignored node (verbose)
    #   c.sh ax "" "" 200        -> raise the node cap (default 120)
    sel="${1:-}"; full="${2:-}"; lim="${3:-}"
    body="$(jq -nc --arg s "$sel" --arg f "$full" --arg l "$lim" \
      '{} + (if $s=="" then {} else {selector:$s} end)
          + (if $f=="" then {} else {full:true} end)
          + (if $l=="" then {} else {limit:($l|tonumber)} end)')"
    resp="$(post ax "$body")"
    if [ -n "$(printf '%s' "$resp" | jq -r '.error // empty')" ]; then
      printf '%s' "$resp" | jq -r '"[ax] ERROR: " + .error'
    else
      printf '%s' "$resp" | jq -r '
        "[ax] " + (.count|tostring) + " nodes" + (if .truncated then " (capped — raise limit)" else "" end),
        (.nodes[] | "  " + .role + ": " + (.name // "")
          + (if .value then " = " + .value else "" end)
          + (if .state then "  [" + .state + "]" else "" end))'
    fi
    ;;
  console)
    # console [level] [limit] [clear] [--all] — raw ring-buffer JSON. Scoped to
    # the CURRENT page generation by default (stale entries from previous
    # loads/instances are counted, not replayed); --all includes them.
    #   c.sh console               -> current-gen console/exception/log events
    #   c.sh console error         -> only errors
    #   c.sh console error 20 1    -> last 20 errors, then clear the buffer
    #   c.sh console "" "" "" --all -> everything ever buffered (stale incl.)
    all=""; args=()
    for a in "$@"; do if [ "$a" = "--all" ]; then all=1; else args+=("$a"); fi; done
    lvl="${args[0]:-}"; lim="${args[1]:-}"; clr="${args[2]:-}"
    cq="$qs"; sep="?"; [ -n "$qs" ] && sep="&"
    [ -n "$lvl" ] && { cq="$cq${sep}level=$lvl"; sep="&"; }
    [ -n "$lim" ] && { cq="$cq${sep}limit=$lim"; sep="&"; }
    [ -n "$clr" ] && { cq="$cq${sep}clear=$clr"; sep="&"; }
    [ -n "$all" ] && { cq="$cq${sep}all=1"; sep="&"; }
    http_get "$API/console$cq"
    ;;
  errors)
    # errors [--all] [limit=20] — the pretty console-error shorthand. Current
    # page generation only by default; --all folds in stale generations too.
    all=""; lim="20"
    for a in "$@"; do if [ "$a" = "--all" ]; then all=1; else lim="$a"; fi; done
    cq="$qs"; sep="?"; [ -n "$qs" ] && sep="&"
    cq="$cq${sep}level=error&limit=$lim"; [ -n "$all" ] && cq="$cq&all=1"
    resp="$(http_get "$API/console$cq")"
    printf '%s' "$resp" | jq -r --arg all "$all" '
      "[errors] " + (.count|tostring) + (if $all == "1" then " (incl. stale gens)" else " current (gen " + ((.gen // 0)|tostring) + ")" end)
        + (if ($all != "1") and ((.stale // 0) > 0) then " · " + (.stale|tostring) + " stale hidden (add --all)" else "" end),
      (.logs[]? | "  ✗ [" + (.kind // "?") + (if .gen != null then "/g" + (.gen|tostring) else "" end) + "] " + ((.text // "?")|.[0:300])
        + (if .url then "  (" + (.url|split("/")|last) + (if .line then ":" + (.line|tostring) else "" end) + ")" else "" end))'
    ;;
  find)
    # find <query> [limit=12] — locate elements by what they SAY (aria-label /
    # visible text / title / placeholder), returns ROBUST selectors + rects.
    # Kills selector guessing: find "Send" then act click on the result.
    q="${1:-}"; lim="${2:-12}"
    if [ -z "$q" ]; then echo "usage: $0 find <text> [limit]" >&2; exit 2; fi
    resp="$(post find "$(jq -nc --arg q "$q" --argjson l "$lim" '{query:$q,limit:$l}')")"
    printf '%s' "$resp" | jq -r --arg q "$q" '
      if .error then "[find] ERROR: " + .error
      else "[find] " + (.count|tostring) + " match(es) for \"" + $q + "\"",
        (.matches[]? | "  " + .selector
          + "   ← " + .tag + (if .role then "[" + .role + "]" else "" end)
          + " \"" + ((.text // "")|.[0:50]) + "\""
          + (if .visible then "" else "  [HIDDEN]" end)
          + (if .disabled then "  [disabled]" else "" end)
          + "  @" + (.rect.x|tostring) + "," + (.rect.y|tostring) + " " + (.rect.w|tostring) + "×" + (.rect.h|tostring))
      end'
    ;;
  text)
    # text [selector] [maxChars=4000] — the page/element as normalized rendered
    # text. Reads EXACT content (transcript, error copy, settings values) for
    # zero image tokens — no screenshot, no ax node caps.
    sel="${1:-}"; max="${2:-4000}"
    body="$(jq -nc --arg s "$sel" --argjson m "$max" '{maxChars:$m} + (if $s=="" then {} else {selector:$s} end)')"
    resp="$(post text "$body")"
    if [ -n "$(printf '%s' "$resp" | jq -r '.error // empty')" ]; then
      printf '%s' "$resp" | jq -r '"[text] ERROR: " + .error,
        (if (.suggestions // []) | length > 0 then "  did you mean:", (.suggestions[] | "    " + .selector + "   ← " + (.text // "")) else empty end)'
    else
      printf '%s' "$resp" | jq -r '"[text] " + (.totalChars|tostring) + " chars" + (if .truncated then " (TRUNCATED to " + ((.text|length)|tostring) + " — raise maxChars)" else "" end), "---", .text'
    fi
    ;;
  look)
    # The verify primitive: page/assistant state + console errors + a screenshot,
    # one round-trip. Prints a human summary then the shot path on the LAST line.
    sel="${1:-}"
    body="$(jq -nc --arg s "$sel" 'if $s=="" then {} else {selector:$s} end')"
    resp="$(post look "$body")"
    printf '%s' "$resp" | jq -r "$LOOK_JQ"'looksum(.)'
    ;;
  peek)
    # peek [selector] — look WITHOUT the screenshot: state + console errors only.
    # Free of image tokens; the right first call for "did that work?" before
    # deciding whether pixels are even needed.
    sel="${1:-}"
    body="$(jq -nc --arg s "$sel" '{noShot:true} + (if $s=="" then {} else {selector:$s} end)')"
    resp="$(post look "$body")"
    printf '%s' "$resp" | jq -r "$LOOK_JQ"'looksum(.)'
    ;;
  act)
    # act <click|key> <arg> [lookSel] [maxSettleMs=1500] — action + settle + look
    # in ONE round-trip. Settle is QUIESCENCE-based now: returns as soon as the
    # DOM stops mutating (~150-400ms typical), capped at maxSettleMs — faster
    # than the old fixed sleep AND never shoots mid-transition. Key combos work:
    # act key "Ctrl+Shift+P". Action errors + selector suggestions print LOUDLY
    # (they used to be silently swallowed — a failed click looked like success).
    av="${1:-}"; arg="${2:-}"; lookSel="${3:-}"; settle="${4:-1500}"
    case "$av" in
      click) actop="$(jq -nc --arg s "$arg" '{op:"click",params:{selector:$s}}')" ;;
      key)   actop="$(jq -nc --arg k "$arg" '{op:"key",params:{key:$k}}')" ;;
      *) echo "usage: $0 act {click|key} <arg> [lookSel] [maxSettleMs]" >&2; exit 2 ;;
    esac
    body="$(jq -nc --argjson act "$actop" --argjson ms "$settle" --arg ls "$lookSel" \
      '{ops:[ $act, {op:"settle",params:{maxMs:$ms,quietMs:120}}, ({op:"look"} + (if $ls=="" then {} else {params:{selector:$ls}} end)) ]}')"
    resp="$(post batch "$body")"
    printf '%s' "$resp" | jq -r --arg av "$av" "$LOOK_JQ$ACT_JQ"'
      .results as $r |
      actsum($r[0]; "act:" + $av),
      settlesum($r[1]),
      looksum($r[-1])'
    ;;
  measure)
    # measure <selector> [nokids] — REAL computed geometry + design tokens for an
    # element and its direct children. The guessing-killer: edit from exact px +
    # resolved CSS vars instead of eyeballing a screenshot.
    #   c.sh measure ".new-chat"          -> box/pad/gap/font/color/bg/border/radius/shadow + kids
    #   c.sh measure ".sidebar" nokids    -> just the element, no children
    sel="${1:-}"; kids="${2:-}"
    if [ -z "$sel" ]; then echo "usage: $0 measure <selector> [nokids|::before|::after]" >&2; exit 2; fi
    peso=""; case "$kids" in ::before|::after|before|after) peso="$kids"; kids="" ;; esac
    body="$(jq -nc --arg s "$sel" --arg p "$peso" --argjson c "$([ "$kids" = "nokids" ] && echo false || echo true)" \
      '{selector:$s,children:$c} + (if $p=="" then {} else {pseudo:$p} end)')"
    resp="$(post measure "$body")"
    if [ -n "$(printf '%s' "$resp" | jq -r '.value.error // .error // empty')" ]; then
      printf '%s' "$resp" | jq -r '"[measure] ERROR: " + (.value.error // .error),
        (if ((.value.suggestions // []) | length) > 0 then "  did you mean:", (.value.suggestions[] | "    " + .selector + "   ← " + (.text // "")) else empty end)'
    else
      printf '%s' "$resp" | jq -r '
        (.value // .) as $v |
        def line(o): "  " + o.tag
          + (if o.hidden then " [display:none — geometry N/A]" else "" end)
          + "  " + (o.box.w|tostring) + "×" + (o.box.h|tostring)
          + (if o.pad then " · pad " + o.pad else "" end)
          + (if o.gap then " · gap " + o.gap else "" end)
          + (if o.margin then " · m " + o.margin else "" end)
          + (if o.font then " · " + o.font else "" end)
          + (if o.color then " · fg " + o.color else "" end)
          + (if o.bg then " · bg " + o.bg else "" end)
          + (if o.border then " · bd " + o.border else "" end)
          + (if o.radius then " · r " + o.radius else "" end)
          + (if o.shadow then " · shadow " + (o.shadow|.[0:60]) else "" end)
          + (if o.opacity then " · op " + o.opacity else "" end)
          + (if o.flex then " · flex " + o.flex else "" end);
        "[measure] " + $v.self.tag, line($v.self),
        (if $v.pseudo then ($v.pseudo[] | "  ┗ " + .tag + "  " + (.box.w|tostring) + "×" + (.box.h|tostring)
          + (if .bg then " · bg " + .bg else "" end) + (if .radius then " · r " + .radius else "" end)
          + (if .color then " · fg " + .color else "" end)) else empty end),
        (if $v.children then "[children " + (($v.children|length)|tostring) + "]" else empty end),
        ($v.children[]? | line(.))'
    fi
    ;;
  eval)
    js="$1"
    post eval "$(jq -nc --arg js "$js" '{js:$js}')"
    ;;
  type)
    sel="$1"; text="$2"; key="${3:-}"
    post type "$(jq -nc --arg s "$sel" --arg t "$text" --arg k "$key" \
      '{selector:$s,text:$t} + (if $k=="" then {} else {key:$k} end)')"
    ;;
  click)
    sel="$1"
    post click "$(jq -nc --arg s "$sel" '{selector:$s}')"
    ;;
  wait)
    # wait <js-expr> [timeoutMs=60000] — poll until truthy. Prints ✓/✗ and exits
    # non-zero on timeout/error so `c.sh wait ... && next` chains honestly.
    js="$1"; t="${2:-60000}"
    resp="$(post wait "$(jq -nc --arg js "$js" --argjson t "$t" '{js:$js,timeoutMs:$t}')")"
    printf '%s' "$resp" | jq -r '
      if .error then "[wait] ✗ " + .error + " (" + ((.elapsedMs // 0)|tostring) + "ms, " + ((.polls // 0)|tostring) + " polls)"
      else "[wait] ✓ " + (.value|tostring|.[0:120]) + "  (" + ((.elapsedMs // 0)|tostring) + "ms, " + ((.polls // 0)|tostring) + " polls)" end'
    printf '%s' "$resp" | jq -e '.error | not' >/dev/null
    ;;
  shot)
    fmt="${1:-jpeg}"; q="${2:-65}"; mode="${3:-path}"
    resp="$(post screenshot "$(jq -nc --arg f "$fmt" --argjson q "$q" '{format:$f,quality:$q}')")"
    if [ "$mode" = "--json" ]; then printf '%s' "$resp"
    else printf '%s' "$resp" | jq -r '.path // (.error | "ERROR: " + .)'; fi
    ;;
  shot-sel)
    # shot-sel <selector> [fmt] [q] [--json|state]
    #   c.sh shot-sel ".new-chat"                 -> clip to selector
    #   c.sh shot-sel ".new-chat" jpeg 70 hover   -> capture the HOVER state (also focus/active)
    sel="$1"; fmt="${2:-jpeg}"; q="${3:-65}"; mode="${4:-path}"
    state=""; case "$mode" in hover|focus|active) state="$mode"; mode="path" ;; esac
    body="$(jq -nc --arg s "$sel" --arg f "$fmt" --argjson q "$q" --arg st "$state" \
      '{selector:$s,format:$f,quality:$q} + (if $st=="" then {} else {state:$st} end)')"
    resp="$(post screenshot "$body")"
    if [ "$mode" = "--json" ]; then printf '%s' "$resp"
    else printf '%s' "$resp" | jq -r '.path // (.error | "ERROR: " + .)'; fi
    ;;
  baseline)
    # baseline [selector] [name] — capture a PNG reference to diff against later.
    # Named refs live in .tmp/base-<name>.png (default name = "sidebar"). Whole-page
    # or selector-clipped. Use BEFORE editing, then `c.sh diff` after each change.
    sel="${1:-}"; name="${2:-sidebar}"
    body="$(jq -nc --arg s "$sel" '{format:"png",quality:0} + (if $s=="" then {} else {selector:$s} end)')"
    resp="$(post screenshot "$body")"
    src="$(printf '%s' "$resp" | jq -r '.path // empty')"
    if [ -z "$src" ]; then printf '%s' "$resp" | jq -r '"[baseline] ERROR: " + (.error // "no shot")'; else
      dest="$(dirname "$src")/base-$name.png"
      cp "$src" "$dest"
      echo "[baseline] $name captured -> $dest"
    fi
    ;;
  diff)
    # diff [selector] [name] [threshold] — pixel-diff the CURRENT view against a
    # saved baseline using pixelmatch's YIQ + anti-aliasing-aware algorithm, so
    # sub-pixel font rendering doesn't read as a change. Reports real changed-pixel
    # %/ratio + the bounding box of what moved + how many AA-edge pixels were
    # suppressed. Catches an unintended change 400px from the edit site instantly.
    #   c.sh baseline ".sidebar"   (before)
    #   c.sh diff ".sidebar"       (after each edit) -> [diff] 2.14% changed · box …
    # threshold is 0–1 (pixelmatch convention, default 0.1; smaller = stricter).
    sel="${1:-}"; name="${2:-sidebar}"; thr="${3:-0.1}"
    base="$(dirname "$0")/.tmp/base-$name.png"
    [ -f "$base" ] || { echo "[diff] no baseline '$name' — run: c.sh baseline \"$sel\" $name" >&2; exit 4; }
    body="$(jq -nc --arg s "$sel" '{format:"png",quality:0} + (if $s=="" then {} else {selector:$s} end)')"
    cur="$(post screenshot "$body" | jq -r '.path // empty')"
    [ -n "$cur" ] || { echo "[diff] current screenshot failed" >&2; exit 5; }
    # Pass FILE PATHS, not base64. The server's vdiff readB64() loads either a
    # data: URL or a disk path, so we hand it the two PNG paths and skip the
    # base64+jq encode step entirely (was ~360ms of pure process-spawn overhead on
    # a tiny image) AND sidestep the ARG_MAX ceiling that inlining the blobs hit.
    # Paths are short, so a normal jq --arg body is safe.
    dbody="$(jq -nc --arg b "$base" --arg a "$cur" --argjson t "$thr" '{before:$b,after:$a,threshold:$t}')"
    resp="$(post vdiff "$dbody")"
    printf '%s' "$resp" | jq -r '
      (.value // .) as $v |
      if $v.error then "[diff] ERROR: " + $v.error
      else "[diff] " + ($v.pct|tostring) + "% changed (" + ($v.changed|tostring) + "/" + ($v.total|tostring) + " px, " + ($v.W|tostring) + "×" + ($v.H|tostring) + ")"
        + (if ($v.aaSkipped // 0) > 0 then "  · " + ($v.aaSkipped|tostring) + " AA-edge px suppressed" else "" end)
        + (if $v.box then "  · region " + ($v.box.w|tostring) + "×" + ($v.box.h|tostring) + " @ (" + ($v.box.x|tostring) + "," + ($v.box.y|tostring) + ")" else "  · IDENTICAL" end)
        + (if $v.sizeMismatch then "\n  ⚠ SIZE MISMATCH: baseline " + ($v.dims.a.w|tostring) + "×" + ($v.dims.a.h|tostring) + " vs current " + ($v.dims.b.w|tostring) + "×" + ($v.dims.b.h|tostring) + " — diffed the overlap only" else "" end)
      end'
    ;;
  batch)
    body="${1:-}"
    if [ -z "$body" ]; then echo "usage: $0 batch '<json-body>'" >&2; exit 2; fi
    post batch "$body"
    ;;
  key)
    k="$1"; mods="${2:-0}"
    post key "$(jq -nc --arg k "$k" --argjson m "${mods:-0}" '{key:$k,modifiers:$m}')"
    ;;
  reload)
    # Hard cache-busting reload — use when HMR wedges on a stale transform.
    post reload "{}"
    ;;
  reset-viewport)
    # Recovery — drop a wedged device-metrics override (innerWidth/Height read
    # tiny after an interrupted shot) without a reload.
    post reset-viewport "{}"
    ;;
  nav)
    # nav <dest> [lookSel] [settleMs] — jump anywhere in ONE call: real clicks by visible label,
    # prerequisites included (`nav games` = Space tab -> More ▾ -> Games), settle, then look.
    #   dests: clean space changed folders files browse games installers types review welcome theme esc
    #   anything else is clicked as a literal visible label:  nav "Rescan"   nav "Scan a folder…"
    # Space views need a drive scan loaded (`c.sh scan C:`); a miss says so. Several surfaces -> `tour`.
    dest="${1:-}"; lookSel="${2:-}"; settle="${3:-1500}"   # settleMs = CAP; returns as soon as DOM + animations are quiet
    if [ -z "$dest" ]; then echo "usage: $0 nav <clean|space|changed|folders|files|browse|games|installers|types|review|welcome|theme|esc|\"Label\"> [lookSel] [settleMs]" >&2; exit 2; fi
    ops="$(nav_ops "$dest")"; n="$(jq 'length' <<<"$ops")"
    body="$(jq -nc --argjson ops "$ops" --argjson ms "$settle" --arg ls "$lookSel" \
      '{ops: ($ops + [ {op:"settle",params:{maxMs:$ms,quietMs:120}}, ({op:"look"} + (if $ls=="" then {} else {params:{selector:$ls}} end)) ])}')"
    resp="$(post batch "$body")"
    printf '%s' "$resp" | jq -r --arg d "$dest" --argjson n "$n" --arg ns "$(nav_needs_scan "$dest")" "$LOOK_JQ$ACT_JQ"'
      .results as $r |
      ( range(0; $n; 2) as $i | actsum($r[$i]; "nav:" + $d + (if $n > 1 then "." + ((($i / 2) + 1) | floor | tostring) else "" end)) ),
      settlesum($r[$n]),
      (if $ns == "1" and (($r[-1].page // {}).spaceLoaded == false) then "[hint] no drive scan loaded — Space views need one: bash scripts/cdp/c.sh scan C:" else empty end),
      looksum($r[-1])'
    ;;
  tour)
    # tour <dest> <dest> ... [--settle N] — visit N surfaces and screenshot EACH, all in ONE round-trip
    # (kills nav -> shot -> nav -> shot). Dests as in `nav`. A failed click is flagged on that surface.
    #   c.sh tour clean space changed games
    settle=1500; args=()
    while [ $# -gt 0 ]; do
      case "$1" in --settle) settle="${2:-1500}"; shift 2 ;; *) args+=("$1"); shift ;; esac
    done
    [ ${#args[@]} -eq 0 ] && { echo "usage: $0 tour <dest> <dest> ... [--settle N]   (dests: see nav)" >&2; exit 2; }
    ops='[]'; plan='[]'
    for d in "${args[@]}"; do
      sq="$(nav_ops "$d" | jq -c --argjson ms "$settle" --arg tag "$d" \
        '. + [ {op:"settle",params:{maxMs:$ms,quietMs:120}}, {op:"screenshot",params:{format:"jpeg",quality:70,_tag:$tag}} ]')"
      ops="$(jq -nc --argjson a "$ops" --argjson b "$sq" '$a + $b')"
      plan="$(jq -nc --argjson p "$plan" --arg d "$d" --argjson n "$(jq 'length' <<<"$sq")" '$p + [{d:$d,n:$n}]')"
    done
    body="$(jq -nc --argjson ops "$ops" '{ops:$ops}')"
    resp="$(post batch "$body")"
    # Each surface owns a slice of the results (its clicks + settle + shot) — index by slice, never by
    # a filtered shot list, which misaligned labels whenever one click failed.
    printf '%s' "$resp" | jq -r --argjson plan "$plan" '
      .results as $r |
      "[tour] " + (($plan|length)|tostring) + " surfaces in ONE round-trip:",
      ( [ foreach $plan[] as $p ({off:0}; {off: (.off + $p.n), start: .off, p: $p}) ][] |
        ($r[.start : (.start + .p.n)]) as $s | ($s[-1]) as $shot |
        ([ $s[:-2][] | select(.error) | .error ] | first) as $bad |
        "  " + .p.d
        + (if $bad then "  ✗ CLICK FAILED: " + ($bad|tostring) + " (shot shows the PREVIOUS surface)" else "" end)
        + "  → " + ($shot.path // ("(shot failed: " + ($shot.error // "?") + ")")) )'
    ;;
  scan)
    # scan <C:> [timeoutSec] — Space tab -> that drive's card -> wait until the scan finishes -> look.
    # Read-only (maps folder sizes). It does save a snapshot, exactly like a scan started from the UI.
    drive="${1:-}"; tmo="${2:-180}"
    if [ -z "$drive" ]; then echo "usage: $0 scan <C:|D:|...> [timeoutSec]" >&2; exit 2; fi
    drive="$(printf '%s' "$drive" | tr '[:lower:]' '[:upper:]')"; drive="${drive%:}:"
    waitjs="(() => { const S = window.__crumb; return !!S && !S.spaceScanning && !!S.spaceResult && S.spaceRoot.toUpperCase().startsWith('$drive'); })()"
    body="$(jq -nc --arg d "$drive" --arg js "$waitjs" --argjson t "$((tmo * 1000))" '{ops:[
      {op:"click",params:{selector:"tab=Space"}}, {op:"settle",params:{maxMs:250,quietMs:100}},
      {op:"click",params:{selector:("button.drive >> " + $d)}},
      {op:"wait",params:{js:$js,timeoutMs:$t,intervalMs:500}},
      {op:"settle",params:{maxMs:1500,quietMs:150}}, {op:"look"} ]}')"
    resp="$(post batch "$body")"
    printf '%s' "$resp" | jq -r "$LOOK_JQ$ACT_JQ"'
      .results as $r |
      actsum($r[2]; "scan:click"),
      (if $r[3].error then "[scan] ✗ " + ($r[3].error|tostring) + " after " + (($r[3].elapsedMs // 0)|tostring) + "ms"
       else "[scan] ✓ finished in " + (($r[3].elapsedMs // 0)|tostring) + "ms" end),
      looksum($r[-1])'
    ;;
  ready)
    # ready [timeoutMs] — block until the app is MOUNTED and IDLE: .shell exists, fonts loaded, categories
    # loaded, and no Clean/Space scan or clean running. Kills the "guess a settle time before look" habit.
    t="${1:-30000}"
    js='(() => {
      if (!document.querySelector(".shell")) return false;
      if (document.fonts && document.fonts.status !== "loaded") return false;
      const S = window.__crumb;
      if (!S) return { mounted: true, hook: false };
      if (!S.cats.length || S.scanning || S.cleaning || S.spaceScanning) return false;
      return { mounted: true, hook: true, tab: S.tab, welcome: S.welcomeOpen };
    })()'
    body="$(jq -nc --arg js "$js" --argjson t "$t" '{js:$js,timeoutMs:$t,intervalMs:200}')"
    resp="$(post wait "$body")"
    printf '%s' "$resp" | jq -r '
      if .error then "[ready] ✗ " + .error
      elif (.value|type)=="object" then "[ready] ✓ app mounted"
        + (if .value.hook == false then " · NO window.__crumb hook (prod build? state is DOM-scrape only)" else " · idle on the " + (.value.tab // "?") + " tab" end)
        + (if .value.welcome then " · WELCOME CARD open" else "" end)
        + "  (" + ((.elapsedMs // 0)|tostring) + "ms, " + ((.polls // 0)|tostring) + " polls)"
      else "[ready] ✗ timed out — app never mounted/idle (" + ((.elapsedMs // 0)|tostring) + "ms)" end'
    ;;
  reap|clean)
    # reap [--all] — kill ORPHANED dev processes that leak after an ungraceful
    # exit (Ctrl+C on tauri dev, VS Code closing its terminal, a hard-kill). These
    # bypass Rift's RunEvent::Exit reap, so WebView2 trees + crumbtrail MCP children
    # linger in Task Manager burning memory. STRICTLY path-scoped: only crumbtrail
    # under the DEV target dir (cargo-targets / src-tauri\target) + EBWebView-Dev
    # webviews + stale vite. NEVER touches the user's installed prod Rift (that
    # lives under %LOCALAPPDATA%\Rift, a different path + user-data-dir).
    #   c.sh reap        — reap orphans, KEEP a live/healthy dev instance (default)
    #   c.sh reap --all  — reap EVERYTHING dev incl. a running instance (full reset)
    all=""; [ "${1:-}" = "--all" ] && all="1"
    powershell -NoProfile -Command "
      \$all = '$all' -eq '1'
      # The live dev instance to preserve (unless --all): the one owning :9222's parent chain, or the windowed one.
      \$keep = @()
      if (-not \$all) {
        \$c = Get-NetTCPConnection -LocalPort 9222 -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
        if (\$c) { \$wv = Get-CimInstance Win32_Process -Filter \"ProcessId=\$(\$c.OwningProcess)\" -ErrorAction SilentlyContinue; if (\$wv) { \$keep += \$wv.ParentProcessId } }
        # also keep the windowed dev app + its whole claude/MCP subtree
        Get-Process crumbtrail -ErrorAction SilentlyContinue | Where-Object { \$_.MainWindowTitle -ne '' -and \$_.Path -like '*cargo-targets*' } | ForEach-Object { \$keep += \$_.Id }
      }
      \$keepSet = @{}; \$keep | ForEach-Object { \$keepSet[\$_] = \$true }
      # Build the keep SUBTREE (a kept app's claude children + their rift MCP grandchildren must survive too)
      if (\$keep.Count -gt 0) {
        \$allProcs = Get-CimInstance Win32_Process
        \$changed = \$true
        while (\$changed) { \$changed = \$false; foreach (\$p in \$allProcs) { if (\$keepSet[\$p.ParentProcessId] -and -not \$keepSet[\$p.ProcessId]) { \$keepSet[\$p.ProcessId] = \$true; \$changed = \$true } } }
      }
      \$killedRift = 0; \$killedWv = 0; \$killedVite = 0
      # 1) orphaned dev crumbtrail.exe (path-scoped, not in keep-subtree)
      Get-CimInstance Win32_Process -Filter \"Name='crumbtrail.exe'\" | Where-Object { (\$_.ExecutablePath -like '*cargo-targets*' -or \$_.ExecutablePath -like '*src-tauri\\target*') -and -not \$keepSet[\$_.ProcessId] } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force -ErrorAction SilentlyContinue; \$killedRift++ }
      # 2) orphaned EBWebView-Dev webview trees (not owned by a kept rift)
      Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object { \$_.CommandLine -like '*Crumbtrail?EBWebView-Dev*' -and -not \$keepSet[\$_.ParentProcessId] } | ForEach-Object { Stop-Process -Id \$_.ProcessId -Force -ErrorAction SilentlyContinue; \$killedWv++ }
      # 3) stale vite on 1420 ONLY if we killed the app that owned it (--all), else leave it
      if (\$all) { try { Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction Stop | ForEach-Object { Stop-Process -Id \$_.OwningProcess -Force -ErrorAction SilentlyContinue; \$killedVite++ } } catch {} }
      \$viteMsg = if (\$all) { ', '+\$killedVite+' vite' } else { '' }
      Write-Output ('[reap] killed '+\$killedRift+' orphan crumbtrail.exe, '+\$killedWv+' EBWebView-Dev webview proc(s)'+\$viteMsg)
      if (-not \$all -and \$keep.Count -gt 0) { Write-Output ('[reap] preserved live dev instance (PID '+(\$keep -join ',')+') + its subtree. Use --all to reap everything.') }
      # Report what remains
      \$rn = @(Get-CimInstance Win32_Process -Filter \"Name='crumbtrail.exe'\" | Where-Object { \$_.ExecutablePath -like '*cargo-targets*' }).Count
      \$wn = @(Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object { \$_.CommandLine -like '*Crumbtrail?EBWebView-Dev*' }).Count
      Write-Output ('[reap] remaining dev: '+\$rn+' crumbtrail, '+\$wn+' webview')
    "
    ;;
  doctor)
    # doctor — diagnose WHY CDP is down and print the exact fix. Runs a layered
    # check: wrapper (9223) -> WebView2 CDP (9222) -> ELEVATION (the #1 killer on
    # WebView2 150.x). Turns a bare "fetch failed" into an actionable next step.
    cdp_host="${RIFT_CDP_HOST:-127.0.0.1}"; cdp_port="${RIFT_CDP_PORT:-9222}"
    api_ok=0; cdp_ok=0
    echo "[doctor] Crumbtrail CDP diagnostic"
    # 1) wrapper on 9223
    if curl -sS --max-time 3 "$API/health" >/dev/null 2>&1; then
      hb="$(curl -sS --max-time 3 "$API/health" 2>/dev/null)"
      if [ -n "$(printf '%s' "$hb" | jq -r 'select(.ok==true) | .ok' 2>/dev/null)" ]; then
        api_ok=1; cdp_ok=1
        echo "  ✓ wrapper (9223): up   ✓ WebView2 CDP ($cdp_port): reachable"
        printf '%s' "$hb" | jq -r '"  ✓ target=" + .target + " url=" + (.url//"?") + " pingMs=" + ((.pingMs//0)|tostring) + " gen=" + ((.gen//0)|tostring)'
        if [ -n "$(printf '%s' "$hb" | jq -r 'select(.viewportSuspect==true) | 1' 2>/dev/null)" ]; then
          echo "  ⚠ viewport-suspect: a capture failed to clear its size override — run: bash scripts/cdp/c.sh reset-viewport"
        fi
      else
        api_ok=1
        echo "  ✓ wrapper (9223): up"
        echo "  ✗ WebView2 CDP ($cdp_port): wrapper is up but can't reach it — $(printf '%s' "$hb" | jq -r '.error // "unknown"')"
      fi
    else
      echo "  ✗ wrapper (9223): NOT running  →  start it:  npm run cdp:serve"
    fi
    # 2) direct WebView2 CDP probe (independent of the wrapper)
    if [ "$cdp_ok" -eq 0 ]; then
      if curl -sS --max-time 3 "http://$cdp_host:$cdp_port/json/version" >/dev/null 2>&1; then
        echo "  ✓ WebView2 CDP ($cdp_port): port IS bound (so the wrapper just needs a (re)start: npm run cdp:serve)"
      else
        echo "  ✗ WebView2 CDP ($cdp_port): port NOT bound"
        # 3) is a dev crumbtrail.exe even running?
        devpids="$(powershell -NoProfile -Command "(Get-CimInstance Win32_Process -Filter \"Name='crumbtrail.exe'\" | Where-Object { \$_.ExecutablePath -like '*cargo-targets*' -or \$_.ExecutablePath -like '*src-tauri\\target*' }).ProcessId -join ','" 2>/dev/null | tr -d '\r')"
        if [ -z "$devpids" ]; then
          echo "     → the dev app isn't running.  Launch it:  pwsh -NoProfile -File scripts/run-dev-deelevated.ps1 -WaitForCdp"
        else
          echo "     → dev app IS running (PID $devpids) but CDP didn't bind. Checking elevation…"
          # 4) ELEVATION — the WebView2 150.x killer
          elev="$(powershell -NoProfile -Command "\$id=[System.Security.Principal.WindowsIdentity]::GetCurrent(); (New-Object System.Security.Principal.WindowsPrincipal(\$id)).IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)" 2>/dev/null | tr -d '\r ')"
          wv_dbg="$(powershell -NoProfile -Command "\$p=Get-CimInstance Win32_Process -Filter \"Name='msedgewebview2.exe'\" | Where-Object { \$_.CommandLine -like '*Crumbtrail?EBWebView-Dev*' -and \$_.CommandLine -notlike '*--type=*' } | Select-Object -First 1; if(\$p){[bool](\$p.CommandLine -match 'remote-debugging-port')}else{'no-webview'}" 2>/dev/null | tr -d '\r ')"
          echo "     · this shell elevated: $elev   · webview has debug-port arg: $wv_dbg"
          if [ "$wv_dbg" = "False" ]; then
            echo "     ┃ DIAGNOSIS: WebView2 launched WITHOUT the debug port. This is the"
            echo "     ┃ WebView2 150.x elevated-process regression (WebView2Feedback#5640)."
            echo "     ┃ FIX — relaunch dev at MEDIUM integrity (kills the stale one first):"
            echo "     ┃   pwsh -NoProfile -File scripts/run-dev-deelevated.ps1 -WaitForCdp"
            echo "     ┃ then: npm run cdp:serve   (wrapper)   &&   bash scripts/cdp/c.sh look"
          fi
        fi
      fi
    fi
    ;;
  shutdown)
    curl -sS -X POST "$API/shutdown" 2>/dev/null || true
    ;;
  *)
    echo "usage: $0 [-t main|browser] {health|doctor|reap|targets|look|peek|act|nav|tour|scan|ready|state|page|ax|find|text|errors|measure|console|eval|type|click|wait|shot|shot-sel|baseline|diff|batch|key|reload|reset-viewport|shutdown} ..." >&2
    exit 2
    ;;
esac
echo
