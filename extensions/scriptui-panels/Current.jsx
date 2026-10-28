// Current portable extension: original work, MIT OR Apache-2.0.
// Public ScriptUI and keyframe scripting APIs only. No CEP or vendor runtime.
var AfterCraftCurrentExtension = (function () {
  var VERSION = "1.0.0", SECTION = "Current Extension", MAX_PRESETS = 64, MAX_KEYS = 8192;
  var MODES = ["bezier", "spring", "elastic", "bounce", "standard"];
  var FAMILIES = ["ease", "linear", "quad", "cubic", "quart", "quint", "sine", "expo", "circ", "back"];
  var DIRECTIONS = ["in", "out", "inout"], FILE_LIMIT = 2 * 1024 * 1024, flowDecoder = null;
  function finite(n) { return typeof n === "number" && isFinite(n); }
  function fail(message) { throw new Error(message); }
  function clamp(n, a, b) { return Math.max(a, Math.min(b, n)); }
  function copyValue(v) { return v instanceof Array ? v.slice() : v; }
  function values(v) { return v instanceof Array ? v : [v]; }
  function config(source) {
    if (source === undefined) source = {};
    if (source === null || typeof source !== "object" || source instanceof Array) fail("Graph configuration must be an object.");
    var allowed = { mode: 1, curve: 1, frequency: 1, damping: 1, amplitude: 1, density: 1, family: 1, direction: 1 }, key;
    for (key in source) if (Object.prototype.hasOwnProperty.call(source, key) && !Object.prototype.hasOwnProperty.call(allowed, key)) fail("Unknown graph configuration field: " + key);
    if (source.curve !== undefined && !(source.curve instanceof Array)) fail("Enter four Bezier numbers.");
    var c = { mode: source.mode === undefined ? "bezier" : source.mode, curve: (source.curve === undefined ? [0.3, 0, 0.7, 1] : source.curve).slice(),
      frequency: source.frequency === undefined ? 3 : source.frequency,
      damping: source.damping === undefined ? 6 : source.damping,
      amplitude: source.amplitude === undefined ? 1 : source.amplitude,
      density: source.density === undefined ? 1 : source.density,
      family: source.family === undefined ? "quad" : source.family,
      direction: source.direction === undefined ? "inout" : source.direction };
    var found = false, i;
    for (i = 0; i < MODES.length; i++) if (c.mode === MODES[i]) found = true;
    if (!found) fail("Choose Bezier, Spring, Elastic or Bounce.");
    if (!(c.curve instanceof Array) || c.curve.length !== 4) fail("Enter four Bezier numbers.");
    for (i = 0; i < 4; i++) if (!finite(c.curve[i])) fail("Bezier values must be finite.");
    if (c.curve[0] < 0 || c.curve[0] > 1 || c.curve[2] < 0 || c.curve[2] > 1) fail("Bezier time handles must be 0 to 1.");
    if (c.curve[1] < -9 || c.curve[1] > 10 || c.curve[3] < -9 || c.curve[3] > 10) fail("Bezier value handles must be -9 to 10.");
    if (!finite(c.frequency) || c.frequency < 0.5 || c.frequency > 12) fail("Frequency must be 0.5 to 12.");
    if (!finite(c.damping) || c.damping < 0.1 || c.damping > 20) fail("Damping must be 0.1 to 20.");
    if (!finite(c.amplitude) || c.amplitude < 0 || c.amplitude > 3) fail("Amplitude must be 0 to 3.");
    if (!finite(c.density) || c.density !== Math.floor(c.density) || c.density < 1 || c.density > 8) fail("Samples/frame must be an integer from 1 to 8.");
    found = false; for (i = 0; i < FAMILIES.length; i++) if (c.family === FAMILIES[i]) found = true;
    if (!found) fail("Unknown standard easing family.");
    found = false; for (i = 0; i < DIRECTIONS.length; i++) if (c.direction === DIRECTIONS[i]) found = true;
    if (!found) fail("Choose In, Out or In-Out.");
    return c;
  }
  function cubic(a, b, t) { var u = 1 - t; return 3 * u * u * t * a + 3 * u * t * t * b + t * t * t; }
  function sample(c, t) {
    if (!finite(t) || t <= 0) return 0;
    if (t >= 1) return 1;
    if (c.mode === "standard") {
      if (c.family === "ease") {
        var ease = c.direction === "in" ? [1 / 3, 0, 2 / 3, 2 / 3] : c.direction === "out" ? [1 / 3, 1 / 3, 2 / 3, 1] : [1 / 3, 0, 2 / 3, 1];
        return bezierSample(ease, t);
      }
      function base(x) {
        if (c.family === "linear") return x;
        if (c.family === "quad") return x * x;
        if (c.family === "cubic") return x * x * x;
        if (c.family === "quart") return x * x * x * x;
        if (c.family === "quint") return x * x * x * x * x;
        if (c.family === "sine") return 1 - Math.cos(Math.PI * x / 2);
        if (c.family === "expo") return x === 0 ? 0 : x === 1 ? 1 : Math.pow(2, 10 * x - 10);
        if (c.family === "circ") return 1 - Math.sqrt(Math.max(0, 1 - x * x));
        return x * x * ((1.70158 + 1) * x - 1.70158);
      }
      return c.direction === "in" ? base(t) : c.direction === "out" ? 1 - base(1 - t) : t < 0.5 ? base(2 * t) / 2 : 1 - base(2 * (1 - t)) / 2;
    }
    if (c.mode === "bezier") return bezierSample(c.curve, t);
    var omega = 2 * Math.PI * c.frequency;
    function response(x) {
      var wave = Math.cos(omega * x);
      if (c.mode === "spring") wave += c.damping / omega * Math.sin(omega * x);
      if (c.mode === "bounce") wave = Math.abs(wave);
      return 1 - Math.exp(-c.damping * x) * wave;
    }
    var normal = response(1);
    if (!finite(normal) || Math.abs(normal) < 1e-9) return t;
    var v = t + (response(t) / normal - t) * c.amplitude;
    if (!finite(v)) return t;
    return c.mode === "bounce" ? clamp(v, 0, 1) : v;
  }
  function bezierSample(curve, t) {
      var lo = 0, hi = 1, i, mid;
      for (i = 0; i < 40; i++) { mid = (lo + hi) / 2; if (cubic(curve[0], curve[2], mid) < t) lo = mid; else hi = mid; }
      return cubic(curve[1], curve[3], (lo + hi) / 2);
  }
  function presets() {
    var records = [["Linear Bezier", [1 / 3, 1 / 3, 2 / 3, 2 / 3]], ["Drift", [0.3, 0, 0.7, 1]],
      ["Accelerate", [0.6, 0, 0.8, 0.4]], ["Settle", [0.2, 0.6, 0.4, 1]],
      ["Snap", [0.78, 0, 0.22, 1]], ["Anticipate", [0.35, -0.45, 0.65, 1]], ["Overshoot", [0.35, 0, 0.65, 1.45]]], out = [], i;
    for (i = 0; i < records.length; i++) out.push({ name: records[i][0], config: config({ curve: records[i][1] }) });
    out.push({ name: "CSS Ease", config: config({ curve: [0.25, 0.1, 0.25, 1] }) });
    for (i = 1; i <= 3; i++) out.push({ name: MODES[i].charAt(0).toUpperCase() + MODES[i].slice(1), config: config({ mode: MODES[i] }) });
    for (i = 0; i < FAMILIES.length; i++) for (var d = 0; d < DIRECTIONS.length; d++) {
      var title = FAMILIES[i].charAt(0).toUpperCase() + FAMILIES[i].slice(1);
      if (DIRECTIONS[d] !== "inout") title += DIRECTIONS[d] === "in" ? " In" : " Out";
      out.push({ name: title, config: config({ mode: "standard", family: FAMILIES[i], direction: DIRECTIONS[d] }) });
    }
    return out;
  }
  function unicodeLength(text) {
    var count = 0, i, code;
    for (i = 0; i < text.length; i++) {
      code = text.charCodeAt(i);
      if (code >= 0xD800 && code <= 0xDBFF) {
        var next = text.charCodeAt(i + 1);
        if (!(next >= 0xDC00 && next <= 0xDFFF)) fail("Text contains an invalid Unicode surrogate.");
        i++;
      } else if (code >= 0xDC00 && code <= 0xDFFF) fail("Text contains an invalid Unicode surrogate.");
      count++;
    }
    return count;
  }
  function cleanName(name) {
    if (typeof name !== "string") fail("A list or graph name must be text.");
    var n = name.replace(/^\s+|\s+$/g, "");
    if (!n.length || unicodeLength(n) > 80) fail("Use a name of 1 to 80 characters.");
    return n;
  }
  function encodePreset(p) {
    var c = config(p.config);
    return ["C2", encodeURIComponent(cleanName(p.name)), c.mode, c.curve.join(","), c.frequency, c.damping, c.amplitude, c.density, c.family, c.direction].join("\t");
  }
  function decodePreset(text) {
    if (typeof text !== "string" || text.length > 1500) fail("Preset record is too large.");
    var p = text.split("\t"), raw, curve = [], i;
    if (!((p.length === 8 && p[0] === "C1") || (p.length === 10 && p[0] === "C2"))) fail("Unrecognized Current preset format.");
    raw = p[3].split(",");
    if (raw.length !== 4) fail("Invalid preset curve.");
    for (i = 0; i < raw.length; i++) { if (!raw[i].replace(/\s/g, "").length) fail("A preset number is missing."); curve.push(Number(raw[i])); }
    for (i = 4; i < 8; i++) if (!p[i].replace(/\s/g, "").length) fail("A preset number is missing.");
    return { name: cleanName(decodeURIComponent(p[1])), config: config({ mode: p[2], curve: curve, frequency: Number(p[4]), damping: Number(p[5]), amplitude: Number(p[6]), density: Number(p[7]), family: p.length === 10 ? p[8] : "quad", direction: p.length === 10 ? p[9] : "inout" }) };
  }
  // Short double-buffered settings records; commit the manifest last. No eval or JSON library.
  function loadLibrary(settings) {
    if (!settings.haveSetting(SECTION, "library")) return [];
    var manifest = settings.getSetting(SECTION, "library"), parts = manifest.split(":"), out = [], i, count;
    if (parts.length !== 2 || (parts[0] !== "A" && parts[0] !== "B")) fail("Current library is unreadable; its settings were preserved.");
    count = Number(parts[1]);
    if (!finite(count) || count !== Math.floor(count) || count < 0 || count > MAX_PRESETS) fail("Current library exceeds its preset limit.");
    for (i = 0; i < count; i++) {
      if (!settings.haveSetting(SECTION, parts[0] + i)) fail("Current library has a missing record; its settings were preserved.");
      out.push(decodePreset(settings.getSetting(SECTION, parts[0] + i)));
    }
    return out;
  }
  function saveLibrary(settings, records) {
    if (!(records instanceof Array) || records.length > MAX_PRESETS) fail("The portable library holds at most 64 presets.");
    var encoded = [], old = settings.haveSetting(SECTION, "library") ? settings.getSetting(SECTION, "library") : "B:0", i;
    if (!/^[AB]:\d+$/.test(old)) fail("Current library is unreadable; repair it before saving.");
    for (i = 0; i < records.length; i++) encoded.push(encodePreset(records[i]));
    var slot = old.charAt(0) === "A" ? "B" : "A";
    for (i = 0; i < encoded.length; i++) settings.saveSetting(SECTION, slot + i, encoded[i]);
    settings.saveSetting(SECTION, "library", slot + ":" + encoded.length);
  }
  function equalValue(a, b) {
    var x = values(a), y = values(b), i;
    if (x.length !== y.length) return false;
    for (i = 0; i < x.length; i++) if (x[i] !== y[i]) return false;
    return true;
  }
  function byteLength(text) {
    var count = 0, i, code;
    for (i = 0; i < text.length; i++) {
      code = text.charCodeAt(i);
      if (code < 0x80) count++;
      else if (code < 0x800) count += 2;
      else if (code >= 0xD800 && code <= 0xDBFF) {
        var next = text.charCodeAt(i + 1);
        if (!(next >= 0xDC00 && next <= 0xDFFF)) fail("Invalid Unicode in graph file.");
        count += 4; i++;
      } else if (code >= 0xDC00 && code <= 0xDFFF) fail("Invalid Unicode in graph file.");
      else count += 3;
      if (count > FILE_LIMIT) fail("Current graph files are limited to 2 MiB.");
    }
    return count;
  }
  // A small strict JSON parser for hosts without JSON. It never evaluates source text.
  function parseJSON(text) {
    if (typeof text !== "string" || text.length > FILE_LIMIT) fail("Current graph files are limited to 2 MiB.");
    byteLength(text);
    var at = 0, nodes = 0;
    function whitespace() { while (at < text.length && (text.charAt(at) === " " || text.charAt(at) === "\t" || text.charAt(at) === "\r" || text.charAt(at) === "\n")) at++; }
    function string() {
      var pieces = [], count = 0; at++;
      while (at < text.length) {
        var ch = text.charAt(at++);
        if (ch === '"') { var result = pieces.join(""); unicodeLength(result); return result; }
        if (ch === "\\") {
          if (at >= text.length) fail("Incomplete JSON string escape.");
          ch = text.charAt(at++);
          if (ch === "u") {
            var hex = text.slice(at, at + 4);
            if (!/^[0-9a-fA-F]{4}$/.test(hex)) fail("Invalid JSON Unicode escape.");
            pieces.push(String.fromCharCode(parseInt(hex, 16))); at += 4;
          } else {
            var escape = { '"': '"', "\\": "\\", "/": "/", b: "\b", f: "\f", n: "\n", r: "\r", t: "\t" };
            if (!Object.prototype.hasOwnProperty.call(escape, ch)) fail("Invalid JSON string escape.");
            pieces.push(escape[ch]);
          }
        } else { if (ch.charCodeAt(0) < 32) fail("Unescaped control character in JSON."); pieces.push(ch); }
        count++; if (count > 65536) fail("A graph-file string is too large.");
      }
      fail("Unterminated JSON string.");
    }
    function value(depth) {
      nodes++; if (nodes > 100000 || depth > 16) fail("Graph-file JSON exceeds its structural limits.");
      whitespace(); var ch = text.charAt(at), result, key;
      if (ch === '"') return string();
      if (ch === "[") {
        result = []; at++; whitespace(); if (text.charAt(at) === "]") { at++; return result; }
        while (true) {
          if (result.length >= 16384) fail("Graph-file array is too large.");
          result.push(value(depth + 1)); whitespace(); ch = text.charAt(at++);
          if (ch === "]") return result;
          if (ch !== ",") fail("Expected a JSON array separator.");
        }
      }
      if (ch === "{") {
        result = {}; at++; whitespace(); if (text.charAt(at) === "}") { at++; return result; }
        while (true) {
          whitespace(); if (text.charAt(at) !== '"') fail("Expected a JSON object key."); key = string();
          if (key === "__proto__" || key === "constructor" || key === "prototype") fail("Unsafe JSON object key.");
          if (Object.prototype.hasOwnProperty.call(result, key)) fail("Duplicate JSON object key: " + key);
          whitespace(); if (text.charAt(at++) !== ":") fail("Expected a JSON colon.");
          result[key] = value(depth + 1); whitespace(); ch = text.charAt(at++);
          if (ch === "}") return result;
          if (ch !== ",") fail("Expected a JSON object separator.");
        }
      }
      var literals = [["true", true], ["false", false], ["null", null]], i;
      for (i = 0; i < literals.length; i++) if (text.slice(at, at + literals[i][0].length) === literals[i][0]) { at += literals[i][0].length; return literals[i][1]; }
      var start = at;
      if (text.charAt(at) === "-") at++;
      if (text.charAt(at) === "0") at++;
      else { if (!/[1-9]/.test(text.charAt(at)) || !text.charAt(at)) fail("Invalid JSON value."); while (/[0-9]/.test(text.charAt(at)) && at < text.length) at++; }
      if (text.charAt(at) === ".") { at++; var fraction = at; while (/[0-9]/.test(text.charAt(at)) && at < text.length) at++; if (at === fraction) fail("Invalid JSON fraction."); }
      if (text.charAt(at) === "e" || text.charAt(at) === "E") { at++; if (text.charAt(at) === "+" || text.charAt(at) === "-") at++; var exponent = at; while (/[0-9]/.test(text.charAt(at)) && at < text.length) at++; if (at === exponent) fail("Invalid JSON exponent."); }
      if (at - start > 128) fail("A JSON number is too long.");
      result = Number(text.slice(start, at)); if (!finite(result)) fail("Graph-file numbers must be finite."); return result;
    }
    whitespace(); if (text.charCodeAt(0) === 0xFEFF) at++;
    var result = value(0); whitespace(); if (at !== text.length) fail("Unexpected data after graph-file JSON."); return result;
  }
  function stringifyJSON(value) {
    var nodes = 0;
    function quoted(text) {
      unicodeLength(text); var out = ['"'], i;
      for (i = 0; i < text.length; i++) {
        var code = text.charCodeAt(i), ch = text.charAt(i);
        if (ch === '"' || ch === "\\") out.push("\\" + ch);
        else if (code < 32 || code > 126) out.push("\\u" + ("0000" + code.toString(16)).slice(-4));
        else out.push(ch);
      }
      out.push('"'); return out.join("");
    }
    function write(v, depth) {
      nodes++; if (nodes > 100000 || depth > 16) fail("Graph data exceeds its structural limits.");
      if (v === null) return "null";
      if (typeof v === "string") return quoted(v);
      if (typeof v === "number") { if (!finite(v)) fail("Graph numbers must be finite."); return String(v); }
      if (typeof v === "boolean") return v ? "true" : "false";
      var parts = [], i, key;
      if (v instanceof Array) { for (i = 0; i < v.length; i++) parts.push(write(v[i], depth + 1)); return "[" + parts.join(",") + "]"; }
      if (typeof v !== "object") fail("Graph data contains an unsupported value.");
      for (key in v) if (Object.prototype.hasOwnProperty.call(v, key)) { if (key === "__proto__" || key === "constructor" || key === "prototype") fail("Unsafe graph key."); parts.push(quoted(key) + ":" + write(v[key], depth + 1)); }
      return "{" + parts.join(",") + "}";
    }
    var result = write(value, 0); if (result.length > FILE_LIMIT) fail("Current graph files are limited to 2 MiB."); return result;
  }
  function exactObject(object, allowed, label) {
    if (object === null || typeof object !== "object" || object instanceof Array) fail(label + " must be an object.");
    for (var key in object) if (Object.prototype.hasOwnProperty.call(object, key) && !Object.prototype.hasOwnProperty.call(allowed, key)) fail("Unknown " + label + " field: " + key);
  }
  function validateDocument(document) {
    exactObject(document, { format: 1, version: 1, lists: 1 }, "graph document");
    if (document.format !== "current.graph-lists" || document.version !== 1) fail("This is not a supported Current graph-list file.");
    if (!(document.lists instanceof Array) || document.lists.length > 32) fail("Current supports at most 32 custom graph lists.");
    var result = { format: "current.graph-lists", version: 1, lists: [] }, listNames = {}, total = 0, i, j;
    for (i = 0; i < document.lists.length; i++) {
      var source = document.lists[i]; exactObject(source, { name: 1, graphs: 1 }, "list");
      var title = cleanName(source.name), id = "$" + title.toLowerCase();
      if (title.toLowerCase() === "defaults" || listNames[id]) fail("List names must be unique; Defaults is read-only.");
      listNames[id] = true;
      if (!(source.graphs instanceof Array) || source.graphs.length > 128) fail("A custom list holds at most 128 graphs.");
      var list = { name: title, graphs: [] }, graphNames = {};
      for (j = 0; j < source.graphs.length; j++) {
        var graph = source.graphs[j]; exactObject(graph, { name: 1, config: 1 }, "graph");
        var name = cleanName(graph.name), gid = "$" + name.toLowerCase(); if (graphNames[gid]) fail("Graph names must be unique within each list."); graphNames[gid] = true;
        if (graph.config === null || typeof graph.config !== "object") fail("Graph config must be an object.");
        list.graphs.push({ name: name, config: config(graph.config) }); total++; if (total > 4096) fail("Current supports at most 4,096 custom graphs.");
      }
      result.lists.push(list);
    }
    return result;
  }
  function decodeCurrent(text) { return validateDocument(parseJSON(text)); }
  function encodeCurrent(document) { return stringifyJSON(validateDocument(document)); }
  function uniqueListName(name, lists) {
    var base = cleanName(name), candidate = base, index = 2, exists = true;
    while (exists) {
      exists = candidate.toLowerCase() === "defaults";
      for (var i = 0; i < lists.length; i++) if (lists[i].name.toLowerCase() === candidate.toLowerCase()) exists = true;
      if (exists) {
        var suffix = " (" + index++ + ")", stem = base;
        while (unicodeLength(stem + suffix) > 80) { var last = stem.charCodeAt(stem.length - 1); stem = stem.slice(0, stem.length - (last >= 0xDC00 && last <= 0xDFFF ? 2 : 1)); }
        candidate = stem + suffix;
        if (index > 4098) fail("Could not create a unique imported list name.");
      }
    }
    return candidate;
  }
  function mergeDocuments(existing, imported) {
    var target = validateDocument(existing), incoming = validateDocument(imported);
    for (var i = 0; i < incoming.lists.length; i++) { var list = incoming.lists[i]; list.name = uniqueListName(list.name, target.lists); target.lists.push(list); }
    target = validateDocument(target); encodeCurrent(target); return target;
  }
  function decodeGraphFile(text) {
    var parsed = parseJSON(text);
    if (parsed && parsed.format === "current.graph-lists") return validateDocument(parsed);
    if (typeof flowDecoder === "function") return validateDocument(flowDecoder(parsed));
    fail("This is not a Current graph-list file. Flow interoperability is not enabled for an unverified format.");
  }
  // Flow 1.5.2 library carrier observed with REA: records {name,value:[x1,y1,x2,y2]}.
  // Legacy `curve`, comma-separated values and name-to-curve maps are accepted as data only.
  function flowDocument(parsed) {
    var records = [], graphs = [], i, key;
    function controls(value) {
      var input = typeof value === "string" ? value.split(",") : value;
      if (!(input instanceof Array) || input.length !== 4) fail("A Flow curve must contain exactly four Bezier values.");
      var curve = [];
      for (var j = 0; j < input.length; j++) {
        var n = input[j];
        if (typeof value === "string") { if (!n.replace(/\s/g, "").length) fail("A Flow curve number is missing."); n = Number(n); }
        if (!finite(n)) fail("Flow Bezier values must be finite numbers."); curve.push(n);
      }
      return config({ mode: "bezier", curve: curve });
    }
    if (parsed instanceof Array) records = parsed;
    else {
      exactObject(parsed, parsed, "legacy Flow map");
      for (key in parsed) if (Object.prototype.hasOwnProperty.call(parsed, key)) records.push({ name: key, value: parsed[key] });
    }
    if (records.length > 128) fail("A Flow import is limited to 128 graphs. Split the library into smaller files.");
    for (i = 0; i < records.length; i++) {
      var record = records[i]; exactObject(record, { _id: 1, name: 1, value: 1, curve: 1 }, "Flow record");
      var hasValue = Object.prototype.hasOwnProperty.call(record, "value"), hasCurve = Object.prototype.hasOwnProperty.call(record, "curve");
      if (!hasValue && !hasCurve) fail("A Flow record has no Bezier values.");
      var c = controls(hasValue ? record.value : record.curve);
      if (hasValue && hasCurve && !equalValue(c.curve, controls(record.curve).curve)) fail("A Flow record contains conflicting curve values.");
      var base = cleanName(record.name), title = base, suffix = 2, collision = true;
      while (collision) {
        collision = false; for (var j = 0; j < graphs.length; j++) if (graphs[j].name.toLowerCase() === title.toLowerCase()) collision = true;
        if (collision) { var ending = " (" + suffix++ + ")", stem = base; while (unicodeLength(stem + ending) > 80) { var last = stem.charCodeAt(stem.length - 1); stem = stem.slice(0, stem.length - (last >= 0xDC00 && last <= 0xDFFF ? 2 : 1)); } title = stem + ending; }
      }
      graphs.push({ name: title, config: c });
    }
    return validateDocument({ format: "current.graph-lists", version: 1, lists: [{ name: "Imported Flow", graphs: graphs }] });
  }
  function decodeFlow(text) { return flowDocument(parseJSON(text)); }
  function flowExport(list) {
    var normalized = validateDocument({ format: "current.graph-lists", version: 1, lists: [list] }).lists[0], records = [], skipped = [];
    for (var i = 0; i < normalized.graphs.length; i++) {
      var graph = normalized.graphs[i];
      if (graph.config.mode !== "bezier") { skipped.push(graph.name); continue; }
      records.push({ name: graph.name, value: graph.config.curve.slice() });
    }
    if (!records.length) fail("This list has no Bezier graphs to export to Flow.");
    return { text: stringifyJSON(records), skipped: skipped };
  }
  function encodeFlow(list) { return flowExport(list).text; }
  function loadGraphLists(settings) {
    if (!settings.haveSetting(SECTION, "graphLists")) {
      // Migration is read-only until the next explicit save; old flat records stay intact.
      return { format: "current.graph-lists", version: 1, lists: [{ name: "Custom", graphs: loadLibrary(settings) }] };
    }
    var manifest = settings.getSetting(SECTION, "graphLists"), parts = manifest.split(":"), chunks = [], count = Number(parts[1]), length = Number(parts[2]);
    if (parts.length !== 3 || (parts[0] !== "A" && parts[0] !== "B") || !finite(count) || count !== Math.floor(count) || count < 1 || count > 1399 || !finite(length) || length !== Math.floor(length) || length < 1 || length > FILE_LIMIT) fail("Current graph-list settings are unreadable; they were preserved.");
    for (var i = 0; i < count; i++) {
      var key = "graphs" + parts[0] + "." + i;
      if (!settings.haveSetting(SECTION, key)) fail("A Current settings chunk is missing; the library was preserved.");
      var chunk = settings.getSetting(SECTION, key); if (chunk.length > 1500) fail("A Current settings chunk is too large."); chunks.push(chunk);
    }
    var text = chunks.join(""); if (text.length !== length) fail("Current settings length is invalid; the library was preserved."); return decodeCurrent(text);
  }
  function saveGraphLists(settings, document) {
    var text = encodeCurrent(document), old = settings.haveSetting(SECTION, "graphLists") ? settings.getSetting(SECTION, "graphLists") : "B:1:1";
    if (!/^[AB]:\d+:\d+$/.test(old)) fail("Current settings are unreadable; repair them before saving.");
    var slot = old.charAt(0) === "A" ? "B" : "A", count = Math.ceil(text.length / 1500);
    for (var i = 0; i < count; i++) settings.saveSetting(SECTION, "graphs" + slot + "." + i, text.slice(i * 1500, (i + 1) * 1500));
    settings.saveSetting(SECTION, "graphLists", slot + ":" + count + ":" + text.length);
    return validateDocument(document);
  }
  function lerp(a, b, t) {
    var x = values(a), y = values(b), out = [], i;
    if (x.length !== y.length) fail("Endpoint value dimensions do not match.");
    for (i = 0; i < x.length; i++) { var v = x[i] + (y[i] - x[i]) * t; if (!finite(v)) fail("Motion exceeds the finite range."); out.push(v); }
    return a instanceof Array ? out : out[0];
  }
  function easeCopy(e) { var out = [], i; for (i = 0; i < e.length; i++) out.push({ speed: e[i].speed, influence: e[i].influence }); return out; }
  function easeNative(e) { var out = [], i; for (i = 0; i < e.length; i++) out.push(new KeyframeEase(e[i].speed, e[i].influence)); return out; }
  function propertyState(p) {
    if (!finite(p.numKeys) || p.numKeys < 0 || p.numKeys !== Math.floor(p.numKeys) || p.numKeys > 32768) fail("This property exceeds the portable keyframe limit.");
    var keys = [], i;
    for (i = 1; i <= p.numKeys; i++) keys.push({ time: p.keyTime(i), value: copyValue(p.keyValue(i)),
      incoming: p.keyInInterpolationType(i), outgoing: p.keyOutInterpolationType(i),
      inEase: easeCopy(p.keyInTemporalEase(i)), outEase: easeCopy(p.keyOutTemporalEase(i)),
      auto: p.keyTemporalAutoBezier(i), continuous: p.keyTemporalContinuous(i), selected: p.keySelected(i),
      label: typeof p.keyLabel === "function" ? p.keyLabel(i) : 0 });
    return keys;
  }
  function sameState(a, b) {
    if (a.length !== b.length) return false;
    var i, j, side;
    for (i = 0; i < a.length; i++) {
      if (a[i].time !== b[i].time || !equalValue(a[i].value, b[i].value) || a[i].incoming !== b[i].incoming || a[i].outgoing !== b[i].outgoing ||
        a[i].auto !== b[i].auto || a[i].continuous !== b[i].continuous || a[i].selected !== b[i].selected || a[i].label !== b[i].label) return false;
      for (side = 0; side < 2; side++) {
        var x = side ? a[i].outEase : a[i].inEase, y = side ? b[i].outEase : b[i].inEase;
        if (x.length !== y.length) return false;
        for (j = 0; j < x.length; j++) if (x[j].speed !== y[j].speed || x[j].influence !== y[j].influence) return false;
      }
    }
    return true;
  }
  function selected(comp, physics) {
    if (!comp || !(comp instanceof CompItem)) fail("Open a composition and select adjacent keyframes.");
    var props = comp.selectedProperties, out = [], total = 0, sourceKeys = 0, i, j;
    if (props.length > 512) fail("Select at most 512 properties at once.");
    for (i = 0; i < props.length; i++) {
      var p = props[i];
      if (p.propertyType !== PropertyType.PROPERTY || p.selectedKeys.length < 2) continue;
      if (!p.canVaryOverTime || !p.isInterpolationTypeValid(KeyframeInterpolationType.BEZIER)) fail("The selected property cannot interpolate numeric motion.");
      if (p.expressionEnabled) fail("Disable the selected property's expression before applying Current.");
      var type = p.propertyValueType;
      if (type !== PropertyValueType.OneD && type !== PropertyValueType.TwoD && type !== PropertyValueType.ThreeD && type !== PropertyValueType.TwoD_SPATIAL && type !== PropertyValueType.ThreeD_SPATIAL)
        fail("Portable Current supports numeric scalar/vector properties; shapes, colors and discrete values need another workflow.");
      if (physics && p.isSpatial) fail("Separate Position dimensions before baking Spring, Elastic or Bounce.");
      var owner = p.propertyGroup(p.propertyDepth);
      if (owner && owner.locked) fail("Unlock the selected layer before applying Current.");
      sourceKeys += p.numKeys;
      if (!finite(sourceKeys) || sourceKeys > 16384) fail("Select properties containing at most 16,384 existing keys in total.");
      var indices = p.selectedKeys.slice(), pairs = [], state = propertyState(p);
      indices.sort(function (a, b) { return a - b; });
      for (j = 0; j + 1 < indices.length; j++) if (indices[j + 1] === indices[j] + 1) {
        var index = indices[j] - 1, a = state[index], b = state[index + 1];
        if (p.isSpatial && (p.keyRoving(indices[j]) || p.keyRoving(indices[j + 1]))) fail("Turn off roving on the selected spatial keys first.");
        if (!finite(a.time) || !finite(b.time) || b.time <= a.time) fail("The selected interval has invalid times.");
        var from = values(a.value), to = values(b.value), d;
        if (from.length !== to.length || !from.length || from.length > 3) fail("Unsupported endpoint dimensions.");
        for (d = 0; d < from.length; d++) if (!finite(from[d]) || !finite(to[d])) fail("Endpoint values must be finite.");
        pairs.push(index); total++;
      }
      if (pairs.length) out.push({ prop: p, source: state, expected: state, pairs: pairs });
    }
    if (!total) fail("Select two adjacent keyframes on an animatable numeric property.");
    if (total > 256) fail("Apply Current to at most 256 intervals at once.");
    return out;
  }
  function spatialLength(p, index, a, b) {
    var out = p.keyOutSpatialTangent(index + 1), incoming = p.keyInSpatialTangent(index + 2), start = values(a), end = values(b), length = 0, prev = start, i, d;
    for (i = 1; i <= 128; i++) {
      var t = i / 128, u = 1 - t, next = [], squared = 0;
      for (d = 0; d < start.length; d++) { var x = u * u * u * start[d] + 3 * u * u * t * (start[d] + out[d]) + 3 * u * t * t * (end[d] + incoming[d]) + t * t * t * end[d]; next.push(x); squared += (x - prev[d]) * (x - prev[d]); }
      length += Math.sqrt(squared); prev = next;
    }
    return length;
  }
  function writeSide(p, index, incoming, outgoing, inEase, outEase) {
    p.setTemporalAutoBezierAtKey(index, false); p.setTemporalContinuousAtKey(index, false);
    p.setInterpolationTypeAtKey(index, incoming, outgoing); p.setTemporalEaseAtKey(index, easeNative(inEase), easeNative(outEase));
  }
  function selectTimes(p, source) {
    var selectedTimes = [], i, j;
    for (i = 0; i < source.length; i++) if (source[i].selected) selectedTimes.push(source[i].time);
    for (i = 1; i <= p.numKeys; i++) {
      var yes = false, time = p.keyTime(i);
      for (j = 0; j < selectedTimes.length; j++) if (Math.abs(selectedTimes[j] - time) < 1e-9) yes = true;
      if (p.keySelected(i) !== yes) p.setSelectedAtKey(i, yes);
    }
  }
  function apply(host, raw, previous) {
    var c = config(raw), comp = host.project.activeItem, physics = c.mode !== "bezier", plans, i, j, generated = 0;
    if (!comp || !(comp instanceof CompItem)) fail("Open a composition first.");
    var approximated = false;
    if (!physics) {
      for (i = 0; i <= 2; i += 2) {
        var bounded = Math.max(0.001, Math.min(0.999, c.curve[i]));
        if (bounded !== c.curve[i]) { c.curve[i] = bounded; approximated = true; }
      }
    }
    if (physics && c.density > 1 && String(host.version).indexOf("EffectCraft") >= 0) fail("This EffectCraft host snaps scripted writes to whole frames. Use Samples/frame = 1.");
    if (physics && previous) {
      if (previous.comp !== comp.id) fail("The motion refinement belongs to another composition. Choose adjacent source keys again.");
      plans = previous.plans;
      if (!(plans instanceof Array) || plans.length > 256) fail("The motion refinement is invalid.");
      for (i = 0; i < plans.length; i++) if (!plans[i].prop.selected || !sameState(propertyState(plans[i].prop), plans[i].expected)) fail("The keys or selection changed after baking. Current preserved those edits; choose adjacent source keys again.");
    } else plans = selected(comp, physics);
    var fps = comp.frameRate;
    if (!finite(fps) || fps < 0.001 || fps > 1000) fail("The composition frame rate is outside the supported range.");
    for (i = 0; i < plans.length; i++) {
      var plan = plans[i], p = plan.prop; plan.writes = [];
      if (p.expressionEnabled || p.propertyGroup(p.propertyDepth).locked) fail("A selected property is now locked or expression-driven.");
      for (j = 0; j < plan.pairs.length; j++) {
        var index = plan.pairs[j], a = plan.source[index], b = plan.source[index + 1], seconds = b.time - a.time;
        if (!physics) {
          if (p.isSpatial && (c.curve[1] < 0 || c.curve[1] > 1 || c.curve[3] < 0 || c.curve[3] > 1)) fail("Separate Position dimensions before applying overshoot.");
          var from = values(a.value), to = values(b.value), outgoing = [], incoming = [], dims = p.isSpatial ? 1 : from.length, d;
          for (d = 0; d < dims; d++) {
            var speed = p.isSpatial ? spatialLength(p, index, a.value, b.value) / seconds : (to[d] - from[d]) / seconds;
            var os = speed * c.curve[1] / c.curve[0], ins = speed * (1 - c.curve[3]) / (1 - c.curve[2]);
            if (!finite(os) || !finite(ins)) fail("The requested Bezier speed exceeds the supported range.");
            outgoing.push({ speed: os, influence: c.curve[0] * 100 }); incoming.push({ speed: ins, influence: (1 - c.curve[2]) * 100 });
          }
          plan.writes.push({ index: index, outEase: outgoing, inEase: incoming });
        } else {
          var rate = fps * c.density, first = Math.floor(a.time * rate + 1e-8) + 1, last = Math.ceil(b.time * rate - 1e-8) - 1;
          if (!finite(first) || !finite(last) || Math.abs(first) > 1e12 || Math.abs(last) > 1e12 || last - first + 1 > MAX_KEYS || generated + Math.max(0, last - first + 1) > MAX_KEYS) fail("One bake is limited to 8,192 generated keys at bounded times. Shorten the interval or reduce Samples/frame.");
          var times = [], vals = [], frame;
          for (frame = first; frame <= last; frame++) {
            var time = frame / rate, value = lerp(a.value, b.value, sample(c, (time - a.time) / seconds)), comps = values(value), k;
            for (k = 0; k < comps.length; k++) if ((p.hasMin && comps[k] < p.minValue) || (p.hasMax && comps[k] > p.maxValue)) fail("The overshoot exceeds a selected property's allowed range. Reduce Amplitude.");
            times.push(time); vals.push(value); generated++;
          }
          if (!times.length) fail("This interval is too short to contain a sampled motion key.");
          plan.writes.push({ from: a.time, to: b.time, times: times, values: vals });
        }
      }
    }
    host.beginUndoGroup(physics ? "Current: Bake Motion" : "Current: Apply Easing");
    try {
      for (i = 0; i < plans.length; i++) {
        var item = plans[i], prop = item.prop;
        if (physics) {
          for (j = prop.numKeys; j >= 1; j--) {
            var kt = prop.keyTime(j), w;
            for (w = 0; w < item.writes.length; w++) if (kt > item.writes[w].from + 1e-9 && kt < item.writes[w].to - 1e-9) { prop.removeKey(j); break; }
          }
          for (j = 0; j < item.writes.length; j++) prop.setValuesAtTimes(item.writes[j].times, item.writes[j].values);
          for (j = 0; j < item.writes.length; j++) {
            var write = item.writes[j], n;
            for (n = 1; n <= prop.numKeys; n++) {
              var time = prop.keyTime(n);
              if (time >= write.from - 1e-9 && time <= write.to + 1e-9) {
                var inType = time > write.from + 1e-9 ? KeyframeInterpolationType.LINEAR : prop.keyInInterpolationType(n);
                var outType = time < write.to - 1e-9 ? KeyframeInterpolationType.LINEAR : prop.keyOutInterpolationType(n);
                writeSide(prop, n, inType, outType, easeCopy(prop.keyInTemporalEase(n)), easeCopy(prop.keyOutTemporalEase(n)));
              }
            }
          }
          selectTimes(prop, item.source);
        } else for (j = 0; j < item.writes.length; j++) {
          var bez = item.writes[j], ai = bez.index + 1, bi = ai + 1;
          var oldIn = easeCopy(prop.keyInTemporalEase(ai)), oldOut = easeCopy(prop.keyOutTemporalEase(bi));
          writeSide(prop, ai, prop.keyInInterpolationType(ai), KeyframeInterpolationType.BEZIER, oldIn, bez.outEase);
          writeSide(prop, bi, KeyframeInterpolationType.BEZIER, prop.keyOutInterpolationType(bi), bez.inEase, oldOut);
        }
        item.expected = propertyState(prop);
      }
    } catch (e) { fail("Current could not finish: " + e.message + ". Undo Current to restore the previous keys."); }
    finally { host.endUndoGroup(); }
    return { comp: comp.id, plans: physics ? plans : null, generatedKeys: generated, properties: plans.length, approximated: approximated };
  }
  // UI is defined below, independently of the host-neutral numerical module.
  function open(thisObject) { return openPanel(thisObject); }
  function openPanel(thisObject) {
    var ui = thisObject instanceof Panel ? thisObject : new Window("palette", "Current", undefined, { resizeable: true });
    var state = config(), recipe = null, document = { format: "current.graph-lists", version: 1, lists: [] }, activeList = -1, page = 0;
    var libraryError = "", libraryRevision = "", drag = -1, progress = 0, task = null, playing = false;
    try { var loaded = loadGraphLists(app.settings); libraryRevision = encodeCurrent(loaded); document = loaded; } catch (e) { libraryError = e.message; }
    ui.orientation = "column"; ui.alignChildren = ["fill", "top"]; ui.spacing = 7; ui.margins = 10;
    var top = ui.add("group"), mode = top.add("dropdownlist", undefined, ["Bezier", "Spring", "Elastic", "Bounce", "Standard"], { name: "currentMode" }); mode.selection = 0;
    var compact = top.add("checkbox", undefined, "Compact", { name: "currentCompact" });
    var graph = ui.add("image", undefined, undefined, { name: "currentGraph" }); graph.preferredSize = [300, 190]; graph.minimumSize = [220, 110];
    var numbers = ui.add("group"), fields = [], labels = ["X1", "Y1", "X2", "Y2"], i;
    for (i = 0; i < 4; i++) { numbers.add("statictext", undefined, labels[i]); var f = numbers.add("edittext", undefined, String(state.curve[i]), { name: "current" + labels[i] }); f.characters = 5; fields.push(f); }
    var motion = ui.add("group"), motionFields = [], motionNames = ["Frequency", "Damping", "Amplitude"], defaults = [state.frequency, state.damping, state.amplitude];
    for (i = 0; i < 3; i++) { motion.add("statictext", undefined, motionNames[i]); var mf = motion.add("edittext", undefined, String(defaults[i]), { name: "current" + motionNames[i] }); mf.characters = 4; motionFields.push(mf); }
    var standardRow = ui.add("group"); standardRow.add("statictext", undefined, "Family");
    var family = standardRow.add("dropdownlist", undefined, ["Ease", "Linear", "Quad", "Cubic", "Quart", "Quint", "Sine", "Expo", "Circ", "Back"], { name: "currentFamily" }); family.selection = 2;
    var direction = standardRow.add("dropdownlist", undefined, ["In", "Out", "In-Out"], { name: "currentDirection" }); direction.selection = 2;
    var densityRow = ui.add("group"); densityRow.add("statictext", undefined, "Samples/frame"); var density = densityRow.add("edittext", undefined, "1", { name: "currentDensity" }); density.characters = 3;
    var previewRow = ui.add("group"), play = previewRow.add("button", undefined, "Play preview", { name: "currentPlay" }), seek = previewRow.add("slider", undefined, 0, 0, 1, { name: "currentSeek" }); seek.preferredSize = [130, 20];
    var actions = ui.add("group"), applyButton = actions.add("button", undefined, "Apply", { name: "currentApply" }), reverse = actions.add("button", undefined, "Reverse", { name: "currentReverse" });
    var newSelection = actions.add("button", undefined, "Use selection", { name: "currentUseSelection" });
    newSelection.helpTip = "Start a new application from the currently selected adjacent keys, after undoing or changing the selection.";
    var live = actions.add("checkbox", undefined, "Live apply", { name: "currentLive" }); live.helpTip = "Apply when a control gesture ends. Each application is one Undo group.";
    var libraryTitle = ui.add("statictext", undefined, "Defaults is read-only. Create a list to save curves.", { multiline: true }); libraryTitle.preferredSize = [300, 28];
    var listRow = ui.add("group"), lists = listRow.add("dropdownlist", undefined, [], { name: "currentList" }); lists.preferredSize = [150, 22];
    var listName = listRow.add("edittext", undefined, "Custom", { name: "currentListName" }); listName.characters = 16;
    var listActions = ui.add("group"), newList = listActions.add("button", undefined, "New list", { name: "currentNewList" });
    var renameList = listActions.add("button", undefined, "Rename list", { name: "currentRenameList" }), deleteList = listActions.add("button", undefined, "Delete list", { name: "currentDeleteList" });
    var reloadLists = listActions.add("button", undefined, "Reload", { name: "currentReloadLists" });
    var libraryRows = ui.add("group"); libraryRows.orientation = "column"; libraryRows.alignChildren = ["fill", "top"];
    var pages = ui.add("group"), previousPage = pages.add("button", undefined, "Previous", { name: "currentPreviousPage" }), pageLabel = pages.add("statictext", undefined, "1/1"), nextPage = pages.add("button", undefined, "Next", { name: "currentNextPage" });
    var savedRow = ui.add("group"), saved = savedRow.add("dropdownlist", undefined, [], { name: "currentSaved" }); saved.preferredSize = [180, 22]; var remove = savedRow.add("button", undefined, "Delete", { name: "currentDelete" });
    var saveRow = ui.add("group"), name = saveRow.add("edittext", undefined, "", { name: "currentName" }); name.characters = 18; var save = saveRow.add("button", undefined, "Save curve", { name: "currentSave" });
    var fileRow = ui.add("group"), filePath = fileRow.add("edittext", undefined, "", { name: "currentFilePath" }); filePath.characters = 27;
    filePath.helpTip = "Optional full path to a Current graph-list file. Leave empty for the host's file dialog.";
    var fileActions = ui.add("group");
    var fileFormat = fileActions.add("dropdownlist", undefined, ["Current .current", "Flow .flow"], { name: "currentFileFormat" }); fileFormat.selection = 0;
    var exportButton = fileActions.add("button", undefined, "Export", { name: "currentExport" }), importButton = fileActions.add("button", undefined, "Import", { name: "currentImport" });
    var status = ui.add("statictext", undefined, libraryError || "Select adjacent keys. Previewing does not change the project.", { name: "currentStatus", multiline: true }); status.preferredSize = [300, 44];
    function report(message) { status.text = String(message); }
    function redraw() { graph.notify("onDraw"); }
    function readFields() {
      var next = { mode: MODES[mode.selection ? mode.selection.index : 0], curve: [], frequency: Number(motionFields[0].text), damping: Number(motionFields[1].text), amplitude: Number(motionFields[2].text), density: Number(density.text),
        family: FAMILIES[family.selection ? family.selection.index : 2], direction: DIRECTIONS[direction.selection ? direction.selection.index : 2] }, j;
      for (j = 0; j < fields.length; j++) next.curve.push(Number(fields[j].text));
      state = config(next); return state;
    }
    function refresh() {
      var j;
      for (j = 0; j < fields.length; j++) fields[j].text = String(state.curve[j]);
      motionFields[0].text = String(state.frequency); motionFields[1].text = String(state.damping); motionFields[2].text = String(state.amplitude); density.text = String(state.density);
      for (j = 0; j < MODES.length; j++) if (state.mode === MODES[j]) mode.selection = j;
      for (j = 0; j < FAMILIES.length; j++) if (state.family === FAMILIES[j]) family.selection = j;
      for (j = 0; j < DIRECTIONS.length; j++) if (state.direction === DIRECTIONS[j]) direction.selection = j;
      numbers.visible = state.mode === "bezier"; standardRow.visible = state.mode === "standard";
      motion.visible = !numbers.visible && !standardRow.visible; densityRow.visible = !numbers.visible; reverse.enabled = numbers.visible;
      graph.preferredSize = [300, compact.value ? 120 : 190]; ui.layout.layout(true); redraw();
    }
    function runApply() {
      try {
        readFields(); var next = apply(app, state, state.mode === "bezier" ? null : recipe);
        recipe = next.plans ? next : null;
        report(next.approximated ? "Easing applied with a small endpoint approximation. The saved curve is unchanged." : state.mode === "bezier" ? "Easing applied. Undo Current restores it." : "Baked " + next.generatedKeys + " keys. Undo Current restores the previous motion.");
      } catch (e) { report(e.message); live.value = false; recipe = null; }
    }
    function usePreset(preset, applyNow) { state = config(preset.config); name.text = preset.name; refresh(); if (applyNow || live.value) runApply(); }
    function edited(applyWhenDone) { try { readFields(); redraw(); if (applyWhenDone && live.value) runApply(); } catch (e) { report(e.message); } }
    function bounds() {
      var lo = 0, hi = 1, j;
      if (state.mode === "bezier") { lo = Math.min(0, state.curve[1], state.curve[3]); hi = Math.max(1, state.curve[1], state.curve[3]); }
      else for (j = 0; j <= 96; j++) { var v = sample(state, j / 96); lo = Math.min(lo, v); hi = Math.max(hi, v); }
      return [lo - 0.12, hi + 0.12];
    }
    function drawCurve(control, c, mini) {
      var g = control.graphics, w = control.size.width, h = control.size.height - (mini ? 16 : 0), extent = mini ? [-0.5, 1.5] : bounds(), pad = mini ? 6 : 16;
      function at(x, y) { return [pad + x * (w - 2 * pad), h - pad - (y - extent[0]) / (extent[1] - extent[0]) * (h - 2 * pad)]; }
      function line(points, color, thickness) {
        var pen = g.newPen(g.PenType.SOLID_COLOR, color, thickness); g.newPath();
        for (var k = 0; k < points.length; k++) { if (!k) g.moveTo(points[k][0], points[k][1]); else g.lineTo(points[k][0], points[k][1]); }
        g.strokePath(pen);
      }
      g.newPath(); g.rectPath(0, 0, w, h); g.fillPath(g.newBrush(g.BrushType.SOLID_COLOR, [0.09, 0.09, 0.1, 1]));
      if (!mini) for (var grid = 0; grid <= 4; grid++) {
        line([at(grid / 4, extent[0]), at(grid / 4, extent[1])], [0.23, 0.23, 0.25, 1], 1);
        line([at(0, grid / 4), at(1, grid / 4)], [0.23, 0.23, 0.25, 1], 1);
      }
      var points = [];
      for (var n = 0; n <= 96; n++) points.push(at(n / 96, sample(c, n / 96)));
      line(points, [0.65, 0.5, 0.94, 1], mini ? 1.5 : 2);
      if (!mini && c.mode === "bezier") for (var side = 0; side < 2; side++) {
        var index = side * 2, handle = at(c.curve[index], c.curve[index + 1]); line([at(side, side), handle], [0.66, 0.66, 0.7, 1], 1);
        g.newPath(); g.ellipsePath(handle[0] - 4, handle[1] - 4, 8, 8); g.fillPath(g.newBrush(g.BrushType.SOLID_COLOR, [0.75, 0.64, 1, 1]));
      }
      if (!mini) { var marker = at(progress, sample(c, progress)); g.newPath(); g.ellipsePath(marker[0] - 3, marker[1] - 3, 6, 6); g.fillPath(g.newBrush(g.BrushType.SOLID_COLOR, [1, 1, 1, 1])); }
    }
    graph.onDraw = function () { drawCurve(this, state, false); };
    graph.addEventListener("mousedown", function (e) {
      if (state.mode !== "bezier" || e.button !== 0) return;
      var extent = bounds(), w = graph.size.width - 32, h = graph.size.height - 32, best = 14 * 14; drag = -1;
      for (var side = 0; side < 2; side++) {
        var ix = side * 2, x = 16 + state.curve[ix] * w, y = graph.size.height - 16 - (state.curve[ix + 1] - extent[0]) / (extent[1] - extent[0]) * h;
        var distance = (e.clientX - x) * (e.clientX - x) + (e.clientY - y) * (e.clientY - y);
        if (distance < best) { best = distance; drag = ix; }
      }
    });
    graph.addEventListener("mousemove", function (e) {
      if (drag < 0) return;
      var extent = bounds(), x = (e.clientX - 16) / Math.max(1, graph.size.width - 32), y = extent[0] + (graph.size.height - 16 - e.clientY) / Math.max(1, graph.size.height - 32) * (extent[1] - extent[0]);
      if (e.shiftKey) { x = Math.round(x * 20) / 20; y = Math.round(y * 20) / 20; }
      state.curve[drag] = clamp(x, 0.001, 0.999); state.curve[drag + 1] = clamp(y, -9, 10);
      if (e.altKey) { var other = drag === 0 ? 2 : 0; state.curve[other] = 1 - state.curve[drag]; state.curve[other + 1] = 1 - state.curve[drag + 1]; }
      for (var j = 0; j < fields.length; j++) fields[j].text = String(Math.round(state.curve[j] * 1000000) / 1000000);
      redraw();
    });
    function endDrag() { if (drag >= 0) { drag = -1; if (live.value) runApply(); } }
    graph.addEventListener("mouseup", endDrag); graph.addEventListener("mouseout", endDrag);
    function addCard(row, preset, index) {
      var card = row.add("image", undefined, undefined, { name: "currentPreset" + index }); card.preferredSize = [90, 64];
      card.onDraw = function () {
        drawCurve(this, preset.config, true);
        var pen = this.graphics.newPen(this.graphics.PenType.SOLID_COLOR, [0.86, 0.86, 0.9, 1], 1);
        this.graphics.drawString(preset.name, pen, 4, this.size.height - 15, ScriptUI.newFont("dialog", "REGULAR", 10));
      };
      card.helpTip = preset.name + ": click to preview, double-click to apply.";
      card.addEventListener("click", function (e) { saved.selection = index; usePreset(preset, e.detail >= 2); });
    }
    var builtins = presets();
    function activeGraphs() { return activeList < 0 ? builtins : document.lists[activeList].graphs; }
    function commit(next) {
      if (libraryError) fail(libraryError);
      if (encodeCurrent(loadGraphLists(app.settings)) !== libraryRevision) fail("Lists changed in another Current window. Click Reload before saving; your working curve is still here.");
      document = saveGraphLists(app.settings, next); libraryRevision = encodeCurrent(document);
    }
    function cloneDocument() { return validateDocument(document); }
    function populateLists() {
      lists.removeAll(); lists.add("item", "Defaults");
      for (var j = 0; j < document.lists.length; j++) lists.add("item", document.lists[j].name);
      if (activeList >= document.lists.length) activeList = document.lists.length - 1;
      lists.selection = activeList + 1;
    }
    function savedList() {
      var graphs = activeGraphs(), pagesCount = Math.max(1, Math.ceil(graphs.length / 9)); page = clamp(page, 0, pagesCount - 1);
      while (libraryRows.children.length) libraryRows.remove(libraryRows.children[0]);
      var row;
      for (var j = page * 9; j < Math.min(graphs.length, page * 9 + 9); j++) { if (j % 3 === 0) row = libraryRows.add("group"); addCard(row, graphs[j], j); }
      saved.removeAll(); for (var k = 0; k < graphs.length; k++) saved.add("item", graphs[k].name);
      pageLabel.text = (page + 1) + "/" + pagesCount; previousPage.enabled = page > 0; nextPage.enabled = page + 1 < pagesCount;
      var editable = activeList >= 0 && !libraryError;
      save.enabled = editable; remove.enabled = editable && graphs.length > 0; renameList.enabled = editable; deleteList.enabled = editable;
      newList.enabled = !libraryError && document.lists.length < 32; importButton.enabled = !libraryError; exportButton.enabled = !libraryError;
      libraryTitle.text = activeList < 0 ? "Defaults is read-only. Create a list to save curves." : document.lists[activeList].name + " — " + graphs.length + "/128 graphs";
      ui.layout.layout(true);
    }
    lists.onChange = function () { activeList = lists.selection ? lists.selection.index - 1 : -1; page = 0; if (activeList >= 0) listName.text = document.lists[activeList].name; savedList(); };
    previousPage.onClick = function () { page--; savedList(); };
    nextPage.onClick = function () { page++; savedList(); };
    newList.onClick = function () {
      try { var next = cloneDocument(), title = cleanName(listName.text); next.lists.push({ name: title, graphs: [] }); commit(next); activeList = document.lists.length - 1; page = 0; populateLists(); savedList(); report("List created. Name your working curve and Save curve."); }
      catch (e) { report(e.message); }
    };
    renameList.onClick = function () {
      try { if (activeList < 0) fail("Defaults cannot be renamed."); var next = cloneDocument(); next.lists[activeList].name = cleanName(listName.text); commit(next); populateLists(); savedList(); report("List renamed."); }
      catch (e) { report(e.message); }
    };
    deleteList.onClick = function () {
      try { if (activeList < 0) fail("Defaults cannot be deleted."); var next = cloneDocument(); next.lists.splice(activeList, 1); commit(next); activeList = Math.min(activeList, document.lists.length - 1); page = 0; populateLists(); savedList(); report("Custom list deleted."); }
      catch (e) { report(e.message); }
    };
    reloadLists.onClick = function () {
      try { var loaded = loadGraphLists(app.settings), revision = encodeCurrent(loaded); document = loaded; libraryRevision = revision; libraryError = ""; populateLists(); savedList(); report("Lists reloaded. The working curve is unchanged."); }
      catch (e) { report(e.message); }
    };
    mode.onChange = function () { try { readFields(); refresh(); if (live.value) runApply(); } catch (e) { report(e.message); } };
    compact.onClick = function () { libraryRows.visible = !compact.value; pages.visible = !compact.value; refresh(); };
    for (i = 0; i < fields.length; i++) { fields[i].onChanging = function () { edited(false); }; fields[i].onChange = function () { edited(true); }; }
    for (i = 0; i < motionFields.length; i++) { motionFields[i].onChanging = function () { edited(false); }; motionFields[i].onChange = function () { edited(true); }; }
    density.onChange = function () { edited(true); }; applyButton.onClick = runApply;
    family.onChange = direction.onChange = function () { edited(true); };
    newSelection.onClick = function () { recipe = null; report("Select adjacent source keys, then Apply. Existing keys were not changed."); };
    reverse.onClick = function () { var c = state.curve; state.curve = [1 - c[2], 1 - c[3], 1 - c[0], 1 - c[1]]; refresh(); if (live.value) runApply(); };
    saved.onChange = function () { var graphs = activeGraphs(); if (saved.selection && graphs[saved.selection.index]) usePreset(graphs[saved.selection.index], false); };
    save.onClick = function () {
      try {
        if (activeList < 0) fail("Defaults is read-only. Create or select a custom list first.");
        readFields(); var next = cloneDocument(), graphs = next.lists[activeList].graphs, title = cleanName(name.text), found = -1, j;
        for (j = 0; j < graphs.length; j++) if (graphs[j].name.toLowerCase() === title.toLowerCase()) found = j;
        var record = { name: title, config: config(state) }; if (found < 0) graphs.push(record); else graphs[found] = record;
        commit(next); savedList(); report("Curve saved in " + document.lists[activeList].name + ".");
      } catch (e) { report(e.message); }
    };
    remove.onClick = function () {
      try { if (activeList < 0) fail("Defaults is read-only."); if (!saved.selection) fail("Choose a saved curve first."); var next = cloneDocument(); next.lists[activeList].graphs.splice(saved.selection.index, 1); commit(next); savedList(); report("Saved curve deleted."); } catch (e) { report(e.message); }
    };
    function chosenFile(exporting) {
      var path = filePath.text.replace(/^\s+|\s+$/g, ""), file = path ? new File(path) : exporting ? File.saveDialog("Export Current graphs to a new file") : File.openDialog("Import a Current JSON or Flow .flow library");
      if (!file) fail("Choose a file or paste its full path in the file field. This host may not provide a script file dialog.");
      return file;
    }
    exportButton.onClick = function () {
      var file = null, opened = false;
      try {
        if (libraryError) fail(libraryError);
        var list = activeList < 0 ? { name: "Defaults Copy", graphs: builtins } : document.lists[activeList];
        var flow = fileFormat.selection && fileFormat.selection.index === 1;
        var exported = flow ? flowExport(list) : { text: encodeCurrent({ format: "current.graph-lists", version: 1, lists: [list] }), skipped: [] };
        var text = exported.text;
        file = chosenFile(true);
        var extension = flow ? ".flow" : ".current";
        if (file.fsName.toLowerCase().slice(-extension.length) !== extension) file = new File(file.fsName + extension);
        if (file.exists) fail("Choose a new export filename; Current preserves existing files.");
        file.encoding = "UTF-8";
        if (!file.open("w")) fail("Cannot open the export file. Check its folder and the host's script file-write permission."); opened = true;
        if (file.write(text) === false) fail("The host could not finish writing the graph export.");
        if (file.close() === false) fail("The host could not close the graph export."); opened = false;
        filePath.text = file.fsName; report(exported.skipped.length ? "Exported compatible curves; skipped " + exported.skipped.length + ": " + exported.skipped.join(", ") : "Graph list exported.");
      } catch (e) { report(e.message); }
      finally { if (opened) file.close(); }
    };
    importButton.onClick = function () {
      var file = null, opened = false;
      try {
        if (libraryError) fail(libraryError); file = chosenFile(false);
        if (!file.exists || !finite(file.length) || file.length > FILE_LIMIT) fail("Choose an existing graph file no larger than 2 MiB.");
        file.encoding = "UTF-8";
        if (!file.open("r")) fail("Cannot read the graph file. Check the host's script file permissions."); opened = true;
        var text = file.read(FILE_LIMIT + 1); file.close(); opened = false;
        var incoming = decodeGraphFile(text), count = document.lists.length;
        if (incoming.lists.length !== 1) fail("Choose a file containing one graph list.");
        var filename = decodeURI(file.name).replace(/\.(current|flow|json)$/i, "");
        incoming.lists[0].name = cleanName(filename);
        commit(mergeDocuments(document, incoming)); activeList = count < document.lists.length ? count : -1; page = 0;
        populateLists(); savedList(); report("Graph lists imported. Existing lists were preserved; name collisions received a numbered suffix.");
      } catch (e) { report(e.message); }
      finally { if (opened) file.close(); }
    };
    var global = $.global, registry = global.__AfterCraftCurrentRegistry;
    if (!registry) { registry = { next: 1, instances: {} }; global.__AfterCraftCurrentRegistry = registry; }
    var instance = registry.next++;
    function stopPreview() { playing = false; if (task !== null) { app.cancelTask(task); task = null; } play.text = "Play preview"; }
    registry.instances[instance] = { tick: function () {
      if (!playing) return;
      try { if (ui.visible === false) { stopPreview(); return; } progress = (progress + 1 / 30) % 1; seek.value = progress; redraw(); }
      catch (e) { stopPreview(); report("Preview stopped: " + e.message); }
    } };
    play.onClick = function () {
      try {
        playing = !playing;
        if (playing) task = app.scheduleTask("$.global.__AfterCraftCurrentRegistry.instances[" + instance + "].tick()", 50, true);
        else if (task !== null) { app.cancelTask(task); task = null; }
        play.text = playing ? "Pause preview" : "Play preview";
      } catch (e) { playing = false; report("Preview scheduling is unavailable: " + e.message); }
    };
    seek.onChanging = function () { progress = seek.value; redraw(); }; seek.onChange = seek.onChanging;
    ui.onClose = function () { stopPreview(); delete registry.instances[instance]; };
    ui.onResize = ui.onResizing = function () { ui.layout.resize(); redraw(); };
    populateLists(); savedList(); refresh();
    if (ui instanceof Window) ui.show(); else ui.layout.layout(true);
    return ui;
  }
  flowDecoder = flowDocument;
  return { version: VERSION, config: config, sample: sample, presets: presets, encodePreset: encodePreset, decodePreset: decodePreset,
    loadLibrary: loadLibrary, saveLibrary: saveLibrary, parseJSON: parseJSON, stringifyJSON: stringifyJSON,
    validateDocument: validateDocument, decodeCurrent: decodeCurrent, encodeCurrent: encodeCurrent,
    loadGraphLists: loadGraphLists, saveGraphLists: saveGraphLists, mergeDocuments: mergeDocuments,
    decodeGraphFile: decodeGraphFile, decodeFlow: decodeFlow, encodeFlow: encodeFlow, flowExport: flowExport,
    setFlowDecoder: function (decoder) { if (decoder !== null && typeof decoder !== "function") fail("A Flow decoder must be a trusted function."); flowDecoder = decoder; },
    apply: apply, open: open };
})();
if (typeof module !== "undefined" && module.exports && typeof app === "undefined") module.exports = AfterCraftCurrentExtension;
else AfterCraftCurrentExtension.open(this);
