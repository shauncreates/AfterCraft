"""Synthetic mapping, preservation and hostile-input checks. No vendor binaries run."""
import base64,json,pathlib,tempfile,unittest
from types import SimpleNamespace as N
import aep2effectcraft as c
import aepx

def fx(name):return N(match_name=name,name=name)
def prop(name,match_name='p',**kwargs):return N(name=name,match_name=match_name,**kwargs)
def catalog(effect):return c.Catalog(dict(preferredEdition='ofx',effects=[effect],nativeProfiles=[]))

class ConverterTests(unittest.TestCase):
    def test_builtin_keylight_wins_over_a_registered_adobe_copy(self):
        native=dict(id='vendor.keylight',name='Keylight [Adobe]',params=[])
        builtin=dict(id='ec.keying.keylight',name='Key Light',params=[])
        cat=c.Catalog(dict(preferredEdition='ofx',effects=[native,builtin],nativeProfiles=[
            dict(match_name='Keylight 906',manifest=native)]))
        self.assertEqual(cat.effect(fx('Keylight 906'))['id'],builtin['id'])

    def test_ofx_preference_falls_back_to_aex_and_uses_disk_identity(self):
        effect=dict(id='vendor.aex.twitch',name='Twitch [Adobe]',params=[
            dict(id='p1',name='Amount',type='slider'),dict(id='p27',name='Amount',type='slider')])
        cat=c.Catalog(dict(preferredEdition='ofx',effects=[effect],nativeProfiles=[
            dict(match_name='Original Twitch',manifest=effect,parameter_match_names={'p1':'Original Twitch-0002','p27':'Original Twitch-0027'})]))
        self.assertEqual(cat.effect(fx('Original Twitch'))['id'],effect['id'])
        mapped,_,_=cat.parameter(fx('Original Twitch'),prop('Amount','Original Twitch-0002'),effect)
        self.assertEqual(mapped['id'],'p1')

    def test_normalized_shake_distance_scales_values_keys_and_speed(self):
        effect=dict(id='ofx.com.genarts.sapphire.distort.s_shake',name='S_Shake [OFX]',
                    params=[dict(id='X_Rand_Amp',name='X Rand Amp',type='slider')],
                    parameterSemantics={'X_Rand_Amp':{'doubleType':'OfxParamDoubleTypeNormalisedX'}})
        cat=catalog(effect)
        key=N(value=192,in_temporal_ease=[N(speed=108,influence=70)],out_temporal_ease=[])
        source=prop('X Rand Amp',value=192,keyframes=[key])
        mapped,_,scale=cat.parameter(fx('S_Shake'),source,effect,(1080,720))
        self.assertEqual(mapped['id'],'X_Rand_Amp')
        proxy=c.ScaledProperty(source,scale)
        self.assertAlmostEqual(proxy.value,192/1080)
        self.assertAlmostEqual(proxy.keyframes[0].value,192/1080)
        self.assertAlmostEqual(proxy.keyframes[0].in_temporal_ease[0].speed,.1)
        self.assertIsNone(cat.parameter(fx('S_Shake'),source,effect)[0])

    def test_verified_plain_sapphire_amount_is_not_left_at_default(self):
        effect=dict(id='ofx.com.genarts.sapphire.distort.s_distort',name='S_Distort [OFX]',
                    params=[dict(id='Amount',name='Amount',type='slider')],
                    parameterSemantics={'Amount':{'doubleType':'OfxParamDoubleTypePlain'}})
        mapped,_,scale=catalog(effect).parameter(fx('S_Distort'),prop('Amount'),effect)
        self.assertEqual(mapped['id'],'Amount');self.assertEqual(scale,1)

    def test_fill_stored_opacity_and_ease_are_percent_scaled(self):
        effect=dict(id='ec.generate.fill',name='Fill',params=[dict(id='opacity',name='Opacity',type='slider')])
        mapped,_,scale=catalog(effect).parameter(fx('ADBE Fill'),prop('Opacity','ADBE Fill-0005',min_value=0,max_value=1),effect)
        self.assertEqual(mapped['id'],'opacity');self.assertEqual(scale,100)
        key=N(value=.75,in_temporal_ease=[N(speed=.5,influence=60)],out_temporal_ease=[])
        self.assertEqual(c.ScaledProperty(N(value=.25,keyframes=[key]),scale).keyframes[0].in_temporal_ease[0].speed,50)

    def test_keylight_kept_crop_boundary_does_not_remove_entire_frame(self):
        effect=dict(id='ec.keying.keylight',name='Key Light',params=[dict(id='sourceCrops/cropRight',name='Right',type='slider')])
        key=N(value=75,in_temporal_ease=[N(speed=-10,influence=60)],out_temporal_ease=[])
        source=prop('Right','Keylight 906-0076',min_value=0,max_value=100,value=100,keyframes=[key])
        mapped,_,scale=catalog(effect).parameter(fx('Keylight 906'),source,effect)
        proxy=c.ScaledProperty(source,scale,mapped['valueOffset'])
        self.assertEqual(proxy.value,0)
        self.assertEqual(proxy.keyframes[0].value,25)
        self.assertEqual(proxy.keyframes[0].in_temporal_ease[0].speed,10)

    def test_reordered_popup_validates_every_key_before_mapping(self):
        effect=dict(id='ec.simulation.shatter',name='Shatter',params=[dict(id='shape/pattern',name='Pattern',type='popup',options=['Bricks','Glass'])])
        source=prop('Pattern','APC Shatter-0006',value=1,keyframes=[N(value=3)],property_parameters=['Glass','Unsupported','Bricks'])
        mapped,_,_=catalog(effect).parameter(fx('APC Shatter'),source,effect)
        proxy=c.EnumProperty(source,mapped['enumSource'],mapped['enumTarget'])
        self.assertEqual(proxy.value,2);self.assertEqual(proxy.keyframes[0].value,1)
        source.keyframes.append(N(value=2))
        self.assertIsNone(catalog(effect).parameter(fx('APC Shatter'),source,effect)[0])

    def test_static_center_xy_maps_both_components_and_y_origin(self):
        from py_aep import PropertyValueType
        semantics={'doubleType':'OfxParamDoubleTypeNormalisedXYAbsolute'}
        effect=dict(id='ofx.com.genarts.sapphire.blur.s_blurmocurves',name='S_BlurMoCurves [OFX]',params=[
            dict(id='Center_x',name='Center X',type='slider'),dict(id='Center_y',name='Center Y',type='slider')],
            parameterSemantics={'Center_x':semantics,'Center_y':semantics})
        source=prop('Center XY',value=[270,180],keyframes=[],is_spatial=True)
        mapped,_,_=catalog(effect).parameter(fx('S_BlurMoCurves'),source,effect,(1080,720))
        self.assertEqual(len(mapped['splitTargets']),2)
        self.assertAlmostEqual(c.ComponentProperty(source,0,1/1080,0,PropertyValueType.OneD).value,.25)
        self.assertAlmostEqual(c.ComponentProperty(source,1,-1/720,1,PropertyValueType.OneD).value,.75)
        source.keyframes=[N(value=[270,180])]
        self.assertIsNone(catalog(effect).parameter(fx('S_BlurMoCurves'),source,effect,(1080,720))[0])

    def test_source_curve_replaces_generated_time_remap_keys(self):
        import _base
        from py_aep import PropertyValueType
        key=N(time=.18248697916666656,value=.5,in_interpolation_type=6612,out_interpolation_type=6613,
              in_temporal_ease=[N(speed=0,influence=33)],out_temporal_ease=[N(speed=1,influence=70)])
        source=prop('Time Remap','ADBE Time Remapping',property_value_type=PropertyValueType.OneD,
                    keyframes=[key],is_modified=True)
        converter=_base.Converter(None,'original.aep')
        _base.js=c.js
        converter.leaf(source,'p','Original fixture',0)
        script='\n'.join(converter.out)
        self.assertIn('p.removeKeys(1, p.numKeys)',script)
        self.assertIn('0.18248697916666656',script)
        self.assertLess(script.index('removeKeys'),script.index('setValuesAtTimes'))
        self.assertLess(script.index('setTemporalEaseAtKey'),script.index('setInterpolationTypeAtKey'))
        self.assertIn('KeyframeInterpolationType.LINEAR, KeyframeInterpolationType.BEZIER',script)

    def test_exposure_uses_explicit_engine_group_path(self):
        effect=dict(id='ec.color.exposure',name='Exposure',params=[dict(id='master/exposure',name='Exposure',type='slider')])
        mapped,_,scale=catalog(effect).parameter(fx('ADBE Exposure2'),prop('Exposure','ADBE Exposure2-0003'),effect)
        self.assertEqual(mapped['id'],'master/exposure');self.assertEqual(scale,1)

    def test_unknown_effect_and_unverified_pixel_units_are_not_guessed(self):
        effect=dict(id='ofx.com.genarts.sapphire.blur.s_glow',name='S_Glow [OFX]',params=[dict(id='Glow_Width',name='Glow Width',type='slider')])
        cat=catalog(effect)
        self.assertIsNone(cat.effect(fx('Unrelated Glow')))
        self.assertIsNone(cat.parameter(fx('S_Glow'),prop('Glow Width'),effect)[0])

    def test_popup_requires_corresponding_options(self):
        effect=dict(id='ofx.com.genarts.sapphire.distort.s_shake',name='S_Shake [OFX]',params=[dict(id='Style',name='Style',type='popup',options=['Normal','Twitchy'])])
        cat=catalog(effect)
        self.assertIsNotNone(cat.parameter(fx('S_Shake'),prop('Style',property_parameters=['Normal','Twitchy']),effect)[0])
        self.assertIsNone(cat.parameter(fx('S_Shake'),prop('Style',property_parameters=['Twitchy','Normal']),effect)[0])

    def test_percent_conversion_scales_values_and_ease_speeds(self):
        effect=dict(id='ofx.com.genarts.sapphire.transitions.s_wipeline',name='S_WipeLine [OFX]',params=[dict(id='Wipe_Amount',name='Wipe Amount',type='slider',max=1.5)])
        source=prop('Wipe Percent',max_value=100)
        _,_,scale=catalog(effect).parameter(fx('S_WipeLine'),source,effect)
        key=N(value=75,in_temporal_ease=[N(speed=50,influence=70)],out_temporal_ease=[N(speed=20,influence=40)])
        mapped=c.ScaledProperty(N(value=50,keyframes=[key]),scale)
        self.assertAlmostEqual(mapped.value,.5);self.assertAlmostEqual(mapped.keyframes[0].value,.75)
        self.assertAlmostEqual(mapped.keyframes[0].in_temporal_ease[0].speed,.5)
        self.assertEqual(mapped.keyframes[0].out_temporal_ease[0].influence,40)

    def test_snapshot_retains_keys_expressions_and_tangents_without_evaluation(self):
        source=prop('Unsupported',value=3,expression='throw new Error("must stay data")',keyframes=[
            N(time=.125,value=[2,3],in_interpolation_type=6613,out_interpolation_type=6614,
              in_temporal_ease=[N(speed=3,influence=70)],out_temporal_ease=[N(speed=4,influence=60)],
              in_spatial_tangent=[-2,1],out_spatial_tangent=[2,-1],roving=True)])
        saved=c.Snapshot().property(source)
        self.assertEqual(saved['expression'],source.expression)
        self.assertEqual(saved['keys'][0]['out_spatial_tangent'],[2,-1])
        self.assertEqual(saved['keys'][0]['in_temporal_ease'][0]['speed'],3)
        self.assertTrue(saved['keys'][0]['roving'])
        group=N(properties=[]);group.properties.append(group)
        with self.assertRaises(c.ConversionError):c.Snapshot().property(group)

    def test_xml_rejects_entities_depth_and_preserves_leaf_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            path=pathlib.Path(directory)/'test.aepx'
            path.write_text('<AfterEffectsProject><data bdata="010203"/></AfterEffectsProject>')
            result=aepx.aepx_to_aep_bytes(path)
            self.assertEqual(result[:4],b'RIFX');self.assertIn(b'data\0\0\0\3\1\2\3\0',result)
            path.write_text('<!DOCTYPE x [<!ENTITY x "test">]><AfterEffectsProject/>')
            with self.assertRaises(ValueError):aepx.aepx_to_aep_bytes(path)
            path.write_text('<AfterEffectsProject>'+'<LIST>'*100+'</LIST>'*100+'</AfterEffectsProject>')
            with self.assertRaises(ValueError):aepx.aepx_to_aep_bytes(path)

    def test_empty_project_keeps_exact_source_archive_and_safe_generated_code(self):
        import py_aep
        with tempfile.TemporaryDirectory() as directory:
            root=pathlib.Path(directory);source=root/'source.aep';script=root/'out.jsx';report=root/'report.json';cat=root/'catalog.json'
            py_aep.new().project.save(source)
            cat.write_text('{"effects":[],"nativeProfiles":[]}')
            c.main([str(source),'-o',str(script),'--catalog',str(cat),'--report',str(report),
                    '--source-project','bad\u2028throw new Error("injected").aep'])
            data=json.loads(report.read_text())
            self.assertEqual(base64.b64decode(data['sourceDocument']['contentBase64']),source.read_bytes())
            self.assertEqual(data['source']['keyframes'],0)
            first_comment=next(line for line in script.read_text().splitlines() if line.startswith('//'))
            self.assertNotIn('injected',first_comment)

if __name__=='__main__':unittest.main()
