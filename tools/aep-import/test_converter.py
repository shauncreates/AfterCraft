"""Synthetic mapping, preservation and hostile-input checks. No vendor binaries run."""
import base64,json,pathlib,tempfile,unittest
from types import SimpleNamespace as N
import aep2effectcraft as c
import aepx

def fx(name):return N(match_name=name,name=name)
def prop(name,match_name='p',**kwargs):return N(name=name,match_name=match_name,**kwargs)
def catalog(effect):return c.Catalog(dict(preferredEdition='ofx',effects=[effect],nativeProfiles=[]))

class ConverterTests(unittest.TestCase):
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
