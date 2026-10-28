"""Built-in import regressions using original synthetic metadata, without vendor code."""
import unittest
from builtin_maps import parameter_id, remap_enum, scalar_scale, scalar_offset


class BuiltinMappingTests(unittest.TestCase):
    def test_duplicate_force_names_have_distinct_stable_paths(self):
        self.assertEqual(parameter_id("APC Shatter", "APC Shatter-0024"), "force1/force1Position")
        self.assertEqual(parameter_id("APC Shatter", "APC Shatter-0032"), "force2/force2Position")
        self.assertEqual(parameter_id("APC Shatter", "APC Shatter-0028"), "force1/force1Radius")
        self.assertEqual(parameter_id("APC Shatter", "APC Shatter-0036"), "force2/force2Radius")
        self.assertEqual(parameter_id("APC Shatter", "APC Shatter-0030"), "force1/force1Strength")
        self.assertEqual(parameter_id("APC Shatter", "APC Shatter-0038"), "force2/force2Strength")

    def test_duplicate_camera_and_keylight_controls_stay_separate(self):
        self.assertEqual(parameter_id("APC Shatter", "APC Shatter-0084"), "cameraPosition/focalLength")
        self.assertEqual(parameter_id("APC Shatter", "APC Shatter-0098"), "cornerPins/focalLength")
        self.assertEqual(parameter_id("Keylight 906", "Keylight 906-0019"), "screenMatte/replaceMethod")
        self.assertEqual(parameter_id("Keylight 906", "Keylight 906-0026"), "insideMask/insideReplaceMethod")
        self.assertEqual(parameter_id("Keylight 906", "Keylight 906-0025"), "insideMask/invertInsideMask")
        self.assertEqual(parameter_id("Keylight 906", "Keylight 906-0033"), "outsideMask/invertOutsideMask")

    def test_unknown_match_ids_are_not_guessed(self):
        for effect, match in [("APC Shatter", "Other-0028"), ("Unknown", "Unknown-0028"),
                              ("APC Shatter", "APC Shatter-9999"), ("APC Shatter", "APC Shatter-28"),
                              ("Keylight 906", "ADBE Effect Mask Opacity")]:
            self.assertIsNone(parameter_id(effect, match))

    def test_pattern_subset_and_changed_order_translate_selected_labels(self):
        source = ["Custom", "(-", "Bricks", "Carpenter's Wheel", "Chevrons", "Crescents", "Eggs", "Glass",
                  "Herringbone 1", "Herringbone 2", "Hexagons", "Js", "Octagons & Squares", "Overlaid Squares",
                  "Planks", "Puzzle", "Rhombi", "Squares", "Squares & Triangles", "Stars & Triangles", "Triangles 1", "Triangles 2"]
        target = ["Bricks", "Glass", "Hexagons", "Squares", "Triangles", "Custom"]
        self.assertEqual(remap_enum(8, source, target), 2)
        self.assertEqual(remap_enum(18, source, target), 4)
        self.assertEqual(remap_enum(1, source, target), 6)
        self.assertIsNone(remap_enum(20, source, target))
        self.assertIsNone(remap_enum(2, source, target))
        self.assertEqual(remap_enum(1, ["Point Source", "Distant Source", "First Comp Light"],
                                    ["Distant Source", "Point Source", "First Comp Light"]), 2)

    def test_animated_enum_requires_every_selected_choice_to_exist(self):
        source = ["Glass", "Squares", "Unsupported Pattern"]
        target = ["Squares", "Glass"]
        self.assertEqual([remap_enum(value, source, target) for value in [1, 2, 1]], [2, 1, 2])
        self.assertIsNone(remap_enum(3, source, target))

    def test_invalid_or_ambiguous_menus_are_not_defaults(self):
        for value in [0, -1, 3, 1.5, float("inf"), float("nan"), True, "1", 10**1000]:
            self.assertIsNone(remap_enum(value, ["A", "B"], ["B", "A"]))
        self.assertIsNone(remap_enum(1, ["A", "A"], ["A"]))
        self.assertIsNone(remap_enum(1, ["A"], ["A", "A"]))
        self.assertIsNone(remap_enum(1, None, ["A"]))
        self.assertEqual(remap_enum(2, ["Source", "Final Result "], [{"label": "Final result"}, {"label": "Source"}]), 1)

    def test_localized_or_duplicate_display_labels_use_stable_identities(self):
        self.assertEqual(parameter_id("ADBE Geometry2", "ADBE Geometry2-0009"), "useCompositionShutterAngle")
        self.assertEqual(parameter_id("ADBE Turbulent Displace", "ADBE Turbulent Displace-0010"), "evolutionOptions/randomSeed")
        self.assertEqual(parameter_id("ADBE Channel Combiner", "ADBE Channel Combiner-0006"), "to")
        self.assertEqual(parameter_id("ADBE Tile", "ADBE Tile-0006"), "mirrorEdges")
        self.assertIsNone(parameter_id("ADBE Samurai", "ADBE Paint Shape"))
        self.assertEqual(parameter_id("ADBE CM WigglePosition", "ADBE CM WigglePosition-0001"), "speed")
        self.assertEqual(parameter_id("ADBE CM WigglePosition", "ADBE CM WigglePosition-0002"), "amount")
        self.assertEqual(scalar_scale("ADBE CM WigglePosition", "ADBE CM WigglePosition-0001", 0, 100), 1)

    def test_stored_opacity_normalizes_from_bounds_never_current_value(self):
        self.assertEqual(scalar_scale("ADBE Fill", "ADBE Fill-0005", 0, 1), 100)
        self.assertEqual(scalar_scale("ADBE Fill", "ADBE Fill-0005", 0, 100), 1)
        self.assertAlmostEqual(127.5 * scalar_scale("ADBE Drop Shadow", "ADBE Drop Shadow-0002", 0, 255), 50)
        self.assertEqual(scalar_scale("ADBE Drop Shadow", "ADBE Drop Shadow-0002", 0, 100), 1)
        self.assertIsNone(scalar_scale("ADBE Fill", "ADBE Fill-0005", 0, None))
        self.assertIsNone(scalar_scale("ADBE Fill", "ADBE Fill-0005", 0, 255))
        self.assertEqual(scalar_scale("ADBE Tint", "ADBE Tint-0003", None, None), 1)

    def test_keylight_far_crop_boundaries_become_removed_amounts(self):
        for match in ["Keylight 906-0076", "Keylight 906-0078"]:
            scale = scalar_scale("Keylight 906", match, 0, 100)
            offset = scalar_offset("Keylight 906", match, 0, 100)
            self.assertEqual(scale * 100 + offset, 0)
            self.assertEqual(scale * 75 + offset, 25)
            self.assertEqual(scale * 0 + offset, 100)
            self.assertIsNone(scalar_scale("Keylight 906", match, None, None))
            self.assertIsNone(scalar_offset("Keylight 906", match, None, None))
        self.assertEqual(scalar_offset("ADBE Fill", "ADBE Fill-0005", 0, 1), 0)


if __name__ == "__main__":
    unittest.main()
