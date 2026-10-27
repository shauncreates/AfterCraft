"""Original variable-font test fixture, MIT OR Apache-2.0; fonttools==4.67.0.

The geometric H is original. All visible ASCII characters map to it: this is a
rendering fixture, not a text font. Both its outline and advance vary with wght.
Build API: https://fonttools.readthedocs.io/en/latest/varLib/index.html
"""
from pathlib import Path
from fontTools.designspaceLib import DesignSpaceDocument, AxisDescriptor, SourceDescriptor
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.varLib import build


def master(width):
    builder = FontBuilder(1000, isTTF=True)
    builder.setupGlyphOrder([".notdef", "space", "H"])
    builder.setupCharacterMap({c: "space" if c == 32 else "H" for c in range(32, 127)})
    pen = TTGlyphPen(None)
    for i, point in enumerate([(50, 0), (150, 0), (150, 300), (width-150, 300),
                               (width-150, 0), (width-50, 0), (width-50, 700),
                               (width-150, 700), (width-150, 400), (150, 400),
                               (150, 700), (50, 700)]):
        (pen.moveTo if i == 0 else pen.lineTo)(point)
    pen.closePath()
    builder.setupGlyf({".notdef": TTGlyphPen(None).glyph(),
                       "space": TTGlyphPen(None).glyph(), "H": pen.glyph()})
    builder.setupHorizontalMetrics({".notdef": (500, 0), "space": (250, 0), "H": (width+50, 50)})
    builder.setupHorizontalHeader(ascent=800, descent=-200)
    builder.setupNameTable({"familyName": "AfterCraft Variable Fixture", "styleName": "Regular",
                            "uniqueFontIdentifier": "AfterCraftVariableFixture-1",
                            "fullName": "AfterCraft Variable Fixture Regular",
                            "psName": "AfterCraftVariableFixture-Regular",
                            "version": "Version 1.0", "licenseDescription": "MIT OR Apache-2.0",
                            "copyright": "Copyright 2026 AfterCraft contributors"})
    builder.setupOS2(sTypoAscender=800, sTypoDescender=-200, usWinAscent=800, usWinDescent=200)
    builder.setupPost()
    builder.setupMaxp()
    builder.font.recalcTimestamp = False
    builder.font["head"].created = builder.font["head"].modified = 2082844800
    return builder.font


space = DesignSpaceDocument()
axis = AxisDescriptor()
axis.name, axis.tag, axis.minimum, axis.default, axis.maximum = "Weight", "wght", 100, 400, 900
space.addAxis(axis)
for weight, width in [(100, 400), (400, 600), (900, 1000)]:
    source = SourceDescriptor()
    source.name, source.location, source.font = f"weight-{weight}", {"Weight": weight}, master(width)
    space.addSource(source)
font, _, _ = build(space)
font.recalcTimestamp = False
font["head"].created = font["head"].modified = 2082844800
font.save(Path(__file__).resolve().parents[1] / "assets/fonts/AfterCraftVariableFixture.ttf")
