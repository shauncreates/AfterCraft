#!/usr/bin/env python3
"""Bounded AE project -> staged EffectCraft script and preservation report.

The parser never executes source expressions. Native effect binaries are never loaded here.
Unmapped data is retained in the report, including the original project bytes for files up to
32 MiB; a mapping is a data/host translation, never AEX binary conversion or activation.
"""
import argparse
import base64
import hashlib
import json
import math
import os
import pathlib
import re
import sys

MAX_SOURCE = 64 * 1024 * 1024
MAX_SCRIPT = 32 * 1024 * 1024
MAX_REPORT = 64 * 1024 * 1024
MAX_NODES = 200_000
MAX_KEYS = 1_000_000
MAX_DEPTH = 48


class ConversionError(Exception):
    pass


def bounded_json(path, limit=MAX_REPORT):
    path = pathlib.Path(path)
    if path.stat().st_size > limit:
        raise ConversionError("JSON input exceeds its size limit")
    with path.open(encoding="utf-8-sig") as stream:
        return json.load(stream)


def js(value):
    """Exact finite JSON doubles; no six-decimal key-time rounding."""
    return json.dumps(value, ensure_ascii=True, allow_nan=False, separators=(",", ":"))


def norm(value):
    return re.sub(r"[^a-z0-9]", "", str(value).casefold())


def finite(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def plain(value, depth=0):
    """Preserve decoded semantic values; the source archive retains opaque unparsed bytes."""
    if depth > 16:
        return {"unexpanded": type(value).__name__}
    if value is None or isinstance(value, (bool, str, int)):
        return value
    if isinstance(value, float):
        return value if math.isfinite(value) else {"nonFinite": repr(value)}
    if isinstance(value, bytes):
        if len(value) > 1024 * 1024:
            return {"sha256": hashlib.sha256(value).hexdigest(), "byteLength": len(value), "inSourceArchive": True}
        return {"base64": base64.b64encode(value).decode("ascii")}
    if isinstance(value, (list, tuple)):
        if len(value) > MAX_NODES:
            raise ConversionError("value array exceeds node limit")
        return [plain(v, depth + 1) for v in value]
    if isinstance(value, dict):
        if len(value) > MAX_NODES:
            raise ConversionError("value object exceeds node limit")
        return {str(k): plain(v, depth + 1) for k, v in value.items()}
    attrs = {}
    for key in ("vertices", "in_tangents", "out_tangents", "closed", "text", "font", "font_size",
                "fill_color", "stroke_color", "comment", "duration", "speed", "influence"):
        try:
            item = getattr(value, key)
        except Exception:
            continue
        if not callable(item):
            attrs[key] = plain(item, depth + 1)
    if attrs:
        return {"type": type(value).__name__, **attrs}
    return {"type": type(value).__name__, "unexpanded": True, "inSourceArchive": True}


def get(obj, name, default=None):
    try:
        return getattr(obj, name)
    except Exception:
        return default


def used(prop):
    return bool(get(prop, "keyframes", []) or get(prop, "expression", "") or get(prop, "is_modified", False))


class Snapshot:
    def __init__(self):
        self.nodes = 0
        self.keys = 0
        self.seen = set()

    def property(self, obj, depth=0):
        self.nodes += 1
        if self.nodes > MAX_NODES or depth > MAX_DEPTH:
            raise ConversionError("project property tree exceeds bounded import limits")
        identity = id(obj)
        if identity in self.seen:
            raise ConversionError("cyclic property tree")
        self.seen.add(identity)
        try:
            record = {"matchName": get(obj, "match_name", ""), "name": get(obj, "name", ""),
                      "propertyType": plain(get(obj, "property_type")), "valueType": plain(get(obj, "property_value_type"))}
            children = get(obj, "properties")
            if children is not None:
                record["children"] = [self.property(c, depth + 1) for c in children]
                return record
            for attr in ("value", "expression", "expression_enabled", "is_modified", "dimensions_separated",
                         "is_separation_leader", "is_separation_follower", "property_parameters", "units_text",
                         "min_value", "max_value"):
                record[attr] = plain(get(obj, attr))
            keys = list(get(obj, "keyframes", []) or [])
            self.keys += len(keys)
            if self.keys > MAX_KEYS:
                raise ConversionError("project keyframe count exceeds import limit")
            record["keys"] = [{name: plain(get(key, name)) for name in
                ("time", "value", "in_interpolation_type", "out_interpolation_type", "in_temporal_ease",
                 "out_temporal_ease", "temporal_continuous", "temporal_auto_bezier", "spatial_continuous",
                 "spatial_auto_bezier", "in_spatial_tangent", "out_spatial_tangent", "roving")} for key in keys]
            return record
        finally:
            self.seen.remove(identity)

    def project(self, project):
        comps = []
        total_layers = 0
        three_d = 0
        for comp in project.compositions:
            layers = []
            for layer in comp.layers:
                total_layers += 1
                if total_layers > 100_000:
                    raise ConversionError("too many project layers")
                attrs = {key: plain(get(layer, key)) for key in
                    ("id", "name", "start_time", "stretch", "in_point", "out_point", "enabled", "three_d_layer",
                     "adjustment_layer", "blending_mode", "auto_orient", "motion_blur", "collapse_transformation")}
                attrs["parentId"] = get(get(layer, "parent"), "id")
                attrs["sourceId"] = get(get(layer, "source"), "id")
                attrs["properties"] = [self.property(p) for p in layer.properties]
                three_d += bool(attrs.get("three_d_layer"))
                layers.append(attrs)
            if len(comps) >= 10_000:
                raise ConversionError("too many project compositions")
            comps.append({key: plain(get(comp, key)) for key in ("id", "name", "width", "height", "duration", "frame_rate")} | {"layers": layers})
        return {"compositions": comps}, {"compositions": len(comps), "layers": total_layers,
            "keyframes": self.keys, "properties": self.nodes, "threeDLayers": three_d}


EXPOSURE = {"ADBE Exposure2-0001": "channels", "ADBE Exposure2-0003": "master/exposure",
    "ADBE Exposure2-0004": "master/offset", "ADBE Exposure2-0005": "master/gamma",
    "ADBE Exposure2-0008": "red/redExposure", "ADBE Exposure2-0009": "red/redOffset", "ADBE Exposure2-0010": "red/redGamma",
    "ADBE Exposure2-0013": "green/greenExposure", "ADBE Exposure2-0014": "green/greenOffset", "ADBE Exposure2-0015": "green/greenGamma",
    "ADBE Exposure2-0018": "blue/blueExposure", "ADBE Exposure2-0019": "blue/blueOffset", "ADBE Exposure2-0020": "blue/blueGamma",
    "ADBE Exposure2-0022": "bypassLinearLight"}

# These controls have the same documented dimensionless/frame/Hz semantics in Sapphire AE/OFX.
# Pixels/normalized distances are deliberately absent. Unknown unit mappings remain in the report.
SAPPHIRE_UNITLESS = {norm(s) for s in (
    "Amplitude", "Frequency", "Phase", "Stillness", "Twitch Frequency", "Drift", "Random Seed", "Seed",
    "Red Amplitude", "Green Amplitude", "Blue Amplitude", "Brightness", "Threshold", "Affect Alpha",
    "Mocha Opacity", "Resize Mocha", "Resize Rel X", "Resize Rel Y", "Dilation Quality", "Mix With Source",
    "Red Shift Frames", "Green Shift Frames", "Blue Shift Frames", "Clamp Chroma", "Frame", "Frame Offset",
    "Speed", "Flicker Frequency", "Amplitude Frequency", "Frequency Rel X", "Frequency Rel Y", "Frequency Rel Z")}


class Catalog:
    def __init__(self, data):
        self.preferred = data.get("preferredEdition")
        effects = data.get("effects", data.get("plugins", []))
        if not isinstance(effects, list) or len(effects) > 5000:
            raise ConversionError("invalid bounded effect catalog")
        self.effects = {e["id"]: e for e in effects if isinstance(e, dict) and isinstance(e.get("id"), str)}
        self.profiles = data.get("nativeProfiles", [])
        if not isinstance(self.profiles, list) or len(self.profiles) > 4096:
            raise ConversionError("invalid native profile catalog")
        # Public native metadata keeps semantic UI types (angle versus ordinary scalar).
        for profile in self.profiles:
            manifest = profile.get("manifest", {})
            effect = self.effects.get(manifest.get("id"))
            if effect:
                typed = {p.get("id"): p for p in manifest.get("params", []) if isinstance(p, dict)}
                for param in effect.get("params", []):
                    if param.get("id") in typed:
                        param.update(typed[param["id"]])
        self.builtin = bounded_json(pathlib.Path(__file__).with_name("builtin_effects.json"))

    def effect(self, source):
        mn = source.match_name
        native = [p["manifest"]["id"] for p in self.profiles if p.get("match_name") == mn and isinstance(p.get("manifest"), dict)]
        adobe = [self.effects[i] for i in native if i in self.effects]
        ofx = [e for e in self.effects.values() if e["id"].startswith("ofx.com.genarts.sapphire.")
               and e.get("name", "").removesuffix(" [OFX]") == mn and mn.startswith("S_")]
        for candidates in ([adobe, ofx] if self.preferred == "adobe" else [ofx, adobe]):
            if len(candidates) == 1:
                return candidates[0]
        target = self.builtin.get(mn)
        if target in self.effects:
            return self.effects[target]
        # Catalogs may explicitly declare a vendor/edition equivalence; never guess one from a label.
        explicit = [e for e in self.effects.values() if mn in e.get("adobeMatchNames", [])]
        return explicit[0] if len(explicit) == 1 else None

    def parameter(self, source_fx, source_prop, target_fx):
        params = target_fx.get("params", [])
        if not isinstance(params, list) or len(params) > 2048:
            raise ConversionError("invalid parameter catalog")
        if source_fx.match_name == "ADBE Exposure2":
            target = EXPOSURE.get(source_prop.match_name)
            found = [p for p in params if p.get("id") == target]
        elif source_fx.match_name.startswith("S_Wipe") and norm(source_prop.name) == "wipepercent" and target_fx["id"].startswith("ofx.com.genarts.sapphire."):
            # Sapphire names the same transition ratio Wipe Percent in AE and Wipe Amount
            # in OFX. AE stores 0..100; OFX documents a normal transition as 0..1.
            found = [p for p in params if p.get("id") == "Wipe_Amount"]
        else:
            found = [p for p in params if norm(p.get("name", "")) == norm(source_prop.name)]
        if len(found) != 1:
            return None, "no unique target parameter", 1.0
        target = found[0]
        kind = target.get("type", "")
        if kind in ("enum", "popup"):
            menu = get(source_prop, "property_parameters")
            options = target.get("options") or target.get("ui", {}).get("options")
            if options:
                options = [x.get("label", "") if isinstance(x, dict) else x for x in options]
            # Built-in params with explicit paths have documented schema but may use abbreviated labels.
            if source_fx.match_name != "ADBE Exposure2" and (not menu or not options or [norm(x) for x in menu] != [norm(x) for x in options]):
                return None, "popup option correspondence unverified", 1.0
            return target, "one-based scripting popup with verified options", 1.0
        if kind in ("checkbox", "bool", "color", "point", "point2", "vec2", "angle"):
            return target, "SDK boolean/color/point/angle semantics", 1.0
        is_adobe = target_fx.get("name", "").endswith(" [Adobe]")
        is_builtin = target_fx["id"].startswith("ec.")
        if is_adobe or is_builtin:
            return target, "same-format property identity", 1.0
        if source_fx.match_name.startswith("S_"):
            name = norm(source_prop.name)
            if name == "wipepercent" and get(source_prop, "max_value") == 100 and target.get("max", 1) >= 1:
                return target, "Sapphire wipe percent /100; ease speeds scaled /100", 0.01
            if name in SAPPHIRE_UNITLESS:
                return target, "Sapphire dimensionless/frame/Hz control", 1.0
        return None, "scalar units not verified across host editions", 1.0


def make_converter(base):
    class MappedConverter(base.Converter):
        def __init__(self, project, source, catalog):
            super().__init__(project, source)
            self.catalog = catalog
            self.mapping = []
            self.output_bytes = 0
            self.walk_nodes = 0

        def emit(self, line, depth=0):
            if depth > MAX_DEPTH:
                raise ConversionError("generated tree exceeds depth limit")
            self.output_bytes += len(line.encode("utf-8")) + 2 * depth + 1
            if self.output_bytes > MAX_SCRIPT:
                raise ConversionError("generated script exceeds 32 MiB")
            # A file name is data, never a generated source-code comment.
            if line.startswith("// Generated by"):
                line = "// Generated AE project import; original data is retained in the import report."
            super().emit(line, depth)

        def folders(self):
            root = self.p.root_folder
            done = set()
            stack = [(root, 0)]
            while stack:
                folder, depth = stack.pop()
                if depth > MAX_DEPTH or folder.id in done:
                    raise ConversionError("cyclic or deeply nested source folders")
                done.add(folder.id)
                children = sorted((i for i in self.p.items.values() if type(i).__name__ == "FolderItem" and i.parent_folder is folder), key=lambda i: i.id)
                for child in children:
                    v = self.var("F")
                    self.item_var[child.id] = v
                    self.emit("var %s = T(%s,function(){return app.project.items.addFolder(%s);});" % (v, js("folder " + child.name), js(child.name)))
                    if folder is not root:
                        pv = self.item_var[folder.id]
                        self.emit("if(%s&&%s)%s.parentFolder=%s;" % (v, pv, v, pv))
                stack.extend((child, depth + 1) for child in reversed(children))

        def walk(self, group, ref, ctx, depth):
            self.walk_nodes += 1
            if self.walk_nodes > MAX_NODES or depth > MAX_DEPTH:
                raise ConversionError("conversion property tree exceeds limits")
            if group.match_name != "ADBE Effect Parade":
                return super().walk(group, ref, ctx, depth)
            for effect in group.properties:
                self.stats["effects"] += 1
                target = self.catalog.effect(effect)
                record = {"sourceMatchName": effect.match_name, "sourceName": effect.name, "context": ctx,
                          "targetId": target["id"] if target else None, "parameters": [], "status": "mapped" if target else "unavailable"}
                self.mapping.append(record)
                if target is None:
                    self.notes.append("%s > %s: no supported registered effect mapping; original effect data retained in report" % (ctx, effect.name))
                    continue
                gv = self.var("G")
                self.emit("var %s=T(%s,function(){return %s.addProperty(%s);});" % (gv, js("add " + ctx + " > " + effect.name), ref, js(target["id"])), depth)
                self.emit("if(%s){" % gv, depth)
                self.step("rename " + effect.name, "%s.name=%s;" % (gv, js(effect.name)), depth + 1)
                if get(effect, "enabled", True) is False:
                    self.step("disable " + effect.name, "%s.enabled=false;" % gv, depth + 1)
                self.effect_leaves(effect, effect, gv, target, record, ctx + " > " + effect.name, depth + 1)
                self.emit("}", depth)

        def effect_leaves(self, effect, group, ref, target, record, ctx, depth):
            self.walk_nodes += 1
            if self.walk_nodes > MAX_NODES or depth > MAX_DEPTH:
                raise ConversionError("effect tree exceeds conversion limits")
            for prop in group.properties:
                if prop.property_type != base.PropertyType.PROPERTY:
                    self.effect_leaves(effect, prop, ref, target, record, ctx, depth + 1)
                    continue
                if get(prop, "property_value_type") in base.NO_VALUE_TYPES:
                    continue
                mapped, why, scale = self.catalog.parameter(effect, prop, target)
                audit = {"sourceMatchName": prop.match_name, "sourceName": prop.name,
                         "targetId": mapped["id"] if mapped else None, "scale": scale, "reason": why,
                         "sourceChanged": used(prop)}
                record["parameters"].append(audit)
                if mapped is None:
                    if used(prop):
                        self.notes.append("%s > %s: %s; source values/keys retained in report" % (ctx, prop.name, why))
                    continue
                proxy = ScaledProperty(prop, scale) if scale != 1.0 else prop
                self.leaf(proxy, "QP(%s,%s)" % (ref, js(mapped["id"])), ctx, depth, force=True)

        def run(self, title):
            # This title also appears in a generated line comment in the base converter.
            # JSON escaping elsewhere does not protect that comment from line terminators.
            title = re.sub(r"[\r\n\u2028\u2029]", " ", title)
            code = super().run(title)
            # QP traverses engine parameter paths (Exposure master/exposure), not AE display indices.
            helper = "function QP(o,n){var a=n.split('/');for(var i=0;o&&i<a.length;i++)o=o.property(a[i]);return o&&typeof o.setValue==='function'?o:null;}\n"
            return helper + code + "\nJSON.stringify({summary:" + js(self.stats) + ",warnings:__all,missingProperties:__missing,mediaFallbacks:__mediaFallbacks});\n"

    return MappedConverter


class ScaledEase:
    def __init__(self, source, scale):
        self.speed = source.speed * abs(scale)
        self.influence = source.influence


class ScaledKey:
    def __init__(self, source, scale):
        self.source = source
        self.scale = scale
    def __getattr__(self, name):
        value = getattr(self.source, name)
        if name == "value":
            return scale_value(value, self.scale)
        if name in ("in_temporal_ease", "out_temporal_ease"):
            return [ScaledEase(e, self.scale) for e in value] if value else value
        return value


def scale_value(value, scale):
    if isinstance(value, (list, tuple)):
        return [scale_value(v, scale) for v in value]
    if not finite(value) or not math.isfinite(scale) or scale == 0:
        raise ConversionError("invalid mapped scalar or unit scale")
    return value * scale


class ScaledProperty:
    def __init__(self, source, scale):
        self.source = source
        self.scale = scale
    def __getattr__(self, name):
        value = getattr(self.source, name)
        if name == "value":
            return scale_value(value, self.scale)
        if name == "keyframes":
            return [ScaledKey(k, self.scale) for k in value]
        return value


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project")
    parser.add_argument("-o", "--output", required=True)
    parser.add_argument("--catalog", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument("--site-packages")
    parser.add_argument("--source-project")
    args = parser.parse_args(argv)
    if args.site_packages:
        sys.path.insert(0, os.path.abspath(args.site_packages))
    sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
    # Import only after the explicit private runtime path has been established.
    import _base as base
    source = pathlib.Path(args.project)
    if not source.is_file() or source.stat().st_size > MAX_SOURCE:
        raise ConversionError("source project missing or exceeds 64 MiB")
    raw = source.read_bytes()
    original = os.path.abspath(args.source_project or args.project)
    if source.suffix.lower() not in (".aep", ".aepx"):
        raise ConversionError("choose an .aep or .aepx project")
    if source.suffix.lower() == ".aep" and (raw[:4] != b"RIFX" or raw[8:12] != b"Egg!"):
        raise ConversionError("invalid AE RIFX project header")
    catalog = Catalog(bounded_json(args.catalog))
    base.js = js
    project = base.load(str(source)).project
    snapshot, counts = Snapshot().project(project)
    converter = make_converter(base)(project, original, catalog)
    code = converter.run(os.path.basename(original))
    if len(code.encode("utf-8")) > MAX_SCRIPT:
        raise ConversionError("script exceeds 32 MiB")
    archive = {"path": original, "sha256": hashlib.sha256(raw).hexdigest(), "byteLength": len(raw)}
    if len(raw) <= 32 * 1024 * 1024:
        archive["contentBase64"] = base64.b64encode(raw).decode("ascii")
    report = {"version": 1, "format": "effectcraft.ae-import-report", "source": counts,
        "sourceDocument": archive, "sourceSnapshot": snapshot, "effectMappings": converter.mapping,
        "converterWarnings": converter.notes, "attempted": converter.stats,
        "limitations": ["Mapping metadata does not establish pixel-identical rendering.",
            "Unknown effects and unverified scalar units are preserved in the source report, not rendered as substitutes.",
            "Some layer styles, custom curves/gradients, Essential Properties, trackers and advanced 3D features require further host support.",
            "Expressions are retained as text; this converter does not evaluate them."]}
    encoded = json.dumps(report, ensure_ascii=True, allow_nan=False, separators=(",", ":"))
    if len(encoded.encode("utf-8")) > MAX_REPORT:
        raise ConversionError("source preservation report exceeds 64 MiB")
    pathlib.Path(args.output).write_text(code, encoding="utf-8", newline="\n")
    pathlib.Path(args.report).write_text(encoded, encoding="utf-8", newline="\n")
    print(json.dumps({"source": counts, "attempted": converter.stats, "warnings": len(converter.notes)}))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ConversionError, OSError, ValueError, RecursionError) as error:
        sys.stderr.write("AE import: " + str(error)[:4096] + "\n")
        raise SystemExit(2)
