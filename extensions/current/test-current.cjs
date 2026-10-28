// Original synthetic fixtures. No Adobe application, SDK implementation, or preset is loaded.
const assert = require('node:assert/strict');
const path = require('node:path');
const fs = require('node:fs');
const current = require(fs.existsSync(path.join(__dirname,'Current.jsx')) ? './Current.jsx' : '../scriptui-panels/Current.jsx');

for (const mode of ['bezier', 'spring', 'elastic', 'bounce']) {
  const c = current.config({mode});
  const response = Array.from({length: 1001}, (_, i) => current.sample(c, i / 1000));
  assert.equal(response[0], 0);
  assert.equal(response.at(-1), 1);
  assert(response.every(Number.isFinite));
  if (mode === 'bounce') {
    assert(response.every(x => x >= 0 && x <= 1));
    assert(response.some((x, i) => i && x < response[i - 1]), 'bounce must rebound');
  } else if (mode !== 'bezier') assert(Math.max(...response) > 1.05, `${mode} must overshoot`);
}
const analytic = current.config({mode:'elastic', frequency:1, damping:Math.log(2)});
assert(Math.abs(current.sample(analytic, .5) - (2 + Math.SQRT2)) < 1e-12);
for (const mode of ['spring','elastic','bounce']) {
  for (const time of [0,.1,.5,.9,1]) assert.equal(current.sample(current.config({mode,amplitude:0}),time),time);
}
for (const bad of [null,{mode:'eval'}, {frequency:Infinity}, {damping:NaN}, {amplitude:-1}, {density:1.5}, {curve:[-1,.5,.7,1]}, {mode:'standard',family:'mystery'}, {direction:null}, {toString:'ignored'}]) {
  assert.throws(() => current.config(bad));
}
for (const preset of current.presets()) assert.deepEqual(current.decodePreset(current.encodePreset(preset)),preset);
const unicode = {name:'A\tcurve\n☂ = / %', config:current.config({mode:'spring'})};
assert.deepEqual(current.decodePreset(current.encodePreset(unicode)),unicode);
assert.throws(() => current.decodePreset('eval(global.danger=true)'));
assert.throws(() => current.decodePreset('x'.repeat(1501)));

const store = new Map();
let failAt = null;
const settings = {
  haveSetting: (section,key) => store.has(`${section}/${key}`),
  getSetting: (section,key) => store.get(`${section}/${key}`),
  saveSetting: (section,key,value) => {
    if (key === failAt) throw new Error('synthetic full disk');
    assert(value.length < 1999,'settings values remain below the older AE size ceiling');
    store.set(`${section}/${key}`,value);
  }
};
current.saveLibrary(settings,[unicode]);
assert.deepEqual(current.loadLibrary(settings),[unicode]);
failAt = 'B0';
assert.throws(() => current.saveLibrary(settings,[current.presets()[0]]));
assert.deepEqual(current.loadLibrary(settings),[unicode],'a failed inactive-slot write preserves the committed library');
failAt = null;
failAt = 'library';
assert.throws(() => current.saveLibrary(settings,[current.presets()[0]]));
assert.deepEqual(current.loadLibrary(settings),[unicode],'failed manifest commit preserves the active slot');
failAt = null;
assert.throws(() => current.saveLibrary(settings,Array(65).fill(unicode)));

// Independent known standard values, symmetry, and exact names for read-only Defaults.
for (const [family,expected] of [['linear',.5],['quad',.25],['cubic',.125],['quart',.0625],['quint',.03125],['sine',1-Math.SQRT1_2],['expo',1/32],['circ',1-Math.sqrt(.75)],['back',-.0876975]]) {
  const c=current.config({mode:'standard',family,direction:'in'});
  assert(Math.abs(current.sample(c,.5)-expected)<1e-9,`${family} has the standard ease-in value`);
  for(const t of [.1,.3,.7,.9]) {
    const incoming=current.sample(c,t);
    const outgoing=current.sample(current.config({...c,direction:'out'}),1-t);
    assert(Math.abs(incoming+outgoing-1)<1e-10);
  }
  assert(Math.abs(current.sample(current.config({...c,direction:'inout'}),.5)-.5)<1e-12);
}
const defaultGraphs=current.presets();
assert.equal(new Set(defaultGraphs.map(x=>x.name.toLowerCase())).size,defaultGraphs.length);
for(const name of ['Ease','Ease In','Ease Out','Quad','Quad In','Quad Out','Cubic','Quart','Quint','Sine','Expo','Circ','Back']) assert(defaultGraphs.some(x=>x.name===name),name);
const ease=current.config({mode:'standard',family:'ease'});
assert(Math.abs(current.sample(ease,.25)-.15625)<1e-10,'Ease matches normalized AE-like zero endpoint speeds at one-third influence');

function doc(lists) {return {format:'current.graph-lists',version:1,lists};}
const graph={name:'My spring',config:current.config({mode:'spring'})};
const library=doc([{name:'Motion',graphs:[graph]},{name:'Empty',graphs:[]}]);
assert.deepEqual(current.decodeCurrent(current.encodeCurrent(library)),library);
const defaultsCopy=doc([{name:'Defaults Copy',graphs:defaultGraphs}]);
assert.deepEqual(current.decodeCurrent(current.encodeCurrent(defaultsCopy)),defaultsCopy);
const merged=current.mergeDocuments(library,doc([{name:'Motion',graphs:[graph]}]));
assert.deepEqual(merged.lists.map(x=>x.name),['Motion','Empty','Motion (2)']);
assert.equal(library.lists.length,2,'merging does not mutate the source document');
for(const bad of [doc([{name:'Defaults',graphs:[]}]),doc([{name:'a',graphs:[]},{name:'A',graphs:[]}]),doc([{name:'a',graphs:[graph,{...graph,name:'MY SPRING'}]}]),doc([{name:'bad',graphs:[{name:'bad',config:{mode:'spring',amplitude:null}}]}]),doc(Array.from({length:33},(_,i)=>({name:`L${i}`,graphs:[]})))]) {
  assert.throws(()=>current.encodeCurrent(bad));
}
for(const text of ['{"format":"current.graph-lists","format":"current.graph-lists","version":1,"lists":[]}', '{"__proto__":{}}','{"constructor":{}}','[1,]','{"x":1,}','{"x":1e999}','{"x":"\\uD800"}','"unterminated','0x10','[true,false] trailing','['.repeat(18)+']'.repeat(18),' '.repeat(2097153)]) assert.throws(()=>current.parseJSON(text),text.slice(0,30));
assert.throws(()=>current.decodeCurrent('{"format":"current.graph-lists","version":1,"lists":[],"toString":0}'));
assert.equal(current.parseJSON('{"name":"\\ud83c\\udf0a"}').name,'🌊');
const longUnicode=doc([{name:'🌊'.repeat(80),graphs:[]}]);
assert.deepEqual(current.decodeCurrent(current.encodeCurrent(longUnicode)),longUnicode);

// Old short records migrate without being changed; new list writes have fault-safe chunks.
assert.deepEqual(current.loadGraphLists(settings).lists[0].graphs,[unicode]);
const oldFlat=store.get('Current Extension/library');
current.saveGraphLists(settings,library);
assert.deepEqual(current.loadGraphLists(settings),library);
assert.equal(store.get('Current Extension/library'),oldFlat);
failAt='graphLists';
assert.throws(()=>current.saveGraphLists(settings,merged));
assert.deepEqual(current.loadGraphLists(settings),library,'failed list manifest preserves committed lists');
failAt=null;
current.saveGraphLists(settings,merged);
assert.deepEqual(current.loadGraphLists(settings),merged);

// Only synthetic Flow records, never a proprietary factory preset table.
const flow='[{"_id":"synthetic-1","name":"Edge overshoot","value":[0,-9,1,10]},{"name":"Edge overshoot","curve":"0.1,0,0.9,1"}]';
const imported=current.decodeFlow(flow);
assert.deepEqual(imported.lists[0].graphs.map(g=>g.name),['Edge overshoot','Edge overshoot (2)']);
assert.deepEqual(imported.lists[0].graphs[0].config.curve,[0,-9,1,10]);
const flowRoundTrip=current.decodeFlow(current.encodeFlow(imported.lists[0]));
assert.deepEqual(flowRoundTrip,imported);
assert.deepEqual(current.decodeGraphFile(flow),imported);
assert.deepEqual(current.decodeFlow('{"Legacy":"0.2,0,0.8,1"}').lists[0].graphs[0].config.curve,[.2,0,.8,1]);
assert.throws(()=>current.encodeFlow(library.lists[0]),/Bezier/);
for(const invalid of ['[{"name":"No curve"}]','[{"name":"Bad","value":[0,0,1]}]','[{"name":"Bad","value":[0,0,1,1],"code":"run()"}]','[{"name":"Bad","value":[0,0,1,11]}]','[{"name":"Bad","value":[0,0,1,1],"curve":[.1,0,.9,1]}]']) assert.throws(()=>current.decodeFlow(invalid));

global.KeyframeInterpolationType = {LINEAR:1, BEZIER:2, HOLD:3};
global.PropertyType = {PROPERTY:1};
global.PropertyValueType = {OneD:1, TwoD:2, ThreeD:3, TwoD_SPATIAL:4, ThreeD_SPATIAL:5};
global.KeyframeEase = function(speed,influence) {
  assert(Number.isFinite(speed)); assert(influence >= .1 && influence <= 100);
  this.speed = speed; this.influence = influence;
};
class Property {
  constructor() {
    this.propertyType=1; this.propertyValueType=1; this.propertyDepth=2;
    this.canVaryOverTime=true; this.isSpatial=false; this.selected=true;
    this.expressionEnabled=false; this.hasMin=false; this.hasMax=false;
    this.owner={locked:false}; this.writes=0;
    this.keys=[this.newKey(0,0,true),this.newKey(1,100,true),this.newKey(2,50,false)];
  }
  newKey(time,value,selected=false) {return {time,value,selected,incoming:1,outgoing:1,inEase:[{speed:0,influence:33}],outEase:[{speed:0,influence:33}],auto:false,continuous:false};}
  get numKeys() {return this.keys.length;}
  get selectedKeys() {return this.keys.flatMap((key,i)=>key.selected?[i+1]:[]);}
  isInterpolationTypeValid() {return true;}
  propertyGroup() {return this.owner;}
  keyTime(i) {return this.keys[i-1].time;}
  keyValue(i) {return this.keys[i-1].value;}
  keyInInterpolationType(i) {return this.keys[i-1].incoming;}
  keyOutInterpolationType(i) {return this.keys[i-1].outgoing;}
  keyInTemporalEase(i) {return this.keys[i-1].inEase;}
  keyOutTemporalEase(i) {return this.keys[i-1].outEase;}
  keyTemporalAutoBezier(i) {return this.keys[i-1].auto;}
  keyTemporalContinuous(i) {return this.keys[i-1].continuous;}
  keySelected(i) {return this.keys[i-1].selected;}
  setTemporalAutoBezierAtKey(i,v) {this.keys[i-1].auto=v;this.writes++;}
  setTemporalContinuousAtKey(i,v) {this.keys[i-1].continuous=v;this.writes++;}
  setInterpolationTypeAtKey(i,a,b) {this.keys[i-1].incoming=a;this.keys[i-1].outgoing=b;this.writes++;}
  setTemporalEaseAtKey(i,a,b) {this.keys[i-1].inEase=a;this.keys[i-1].outEase=b;this.writes++;}
  setSelectedAtKey(i,v) {this.keys[i-1].selected=v;}
  removeKey(i) {this.keys.splice(i-1,1);this.writes++;}
  setValuesAtTimes(times,values) {times.forEach((time,i)=>this.keys.push(this.newKey(time,values[i])));this.keys.sort((a,b)=>a.time-b.time);this.writes++;}
}
global.CompItem = class {constructor(property) {this.id=1;this.frameRate=25;this.selectedProperties=[property];}};
function host(property,version='26.0') {
  return {version,project:{activeItem:new CompItem(property)},begin:0,end:0,
    beginUndoGroup(){this.begin++;},endUndoGroup(){this.end++;}};
}
let p = new Property(), h = host(p), untouched = structuredClone(p.keys[2]);
current.apply(h,{curve:[.6,0,.8,.4]});
assert.equal(p.keys.length,3);
assert.equal(p.keys[0].outgoing,2);
assert.equal(p.keys[0].outEase[0].influence,60);
assert.deepEqual(p.keys[2],untouched,'unselected later keys stay intact');
assert.equal(h.begin,1);assert.equal(h.end,1);

for (const mode of ['spring','elastic','bounce']) {
  p = new Property();h=host(p);
  const recipe=current.apply(h,{mode});
  assert.equal(recipe.generatedKeys,24);
  assert.equal(p.keys.length,27);
  assert.equal(p.keys[0].value,0);assert.equal(p.keys[25].value,100);assert.equal(p.keys[26].value,50);
  const next=current.apply(h,{mode,frequency:5},recipe);
  assert.equal(p.keys.length,27,'refinement replaces samples');
  assert.equal(next.generatedKeys,24);
  p.keys[3].value+=7;
  const changed=JSON.stringify(p.keys);
  assert.throws(()=>current.apply(h,{mode,frequency:4},next),/preserved/);
  assert.equal(JSON.stringify(p.keys),changed,'an external key edit blocks replacement');
}
p=new Property();h=host(p,'26.0 (EffectCraft 0.16.0)');
assert.throws(()=>current.apply(h,{mode:'spring',density:2}),/whole frames/);
assert.equal(p.writes,0);assert.equal(h.begin,0);
p=new Property();p.expressionEnabled=true;h=host(p);
assert.throws(()=>current.apply(h,{mode:'spring'}),/expression/);
assert.equal(p.writes,0);
p=new Property();p.isSpatial=true;h=host(p);
assert.throws(()=>current.apply(h,{mode:'spring'}),/Separate Position/);
assert.equal(p.writes,0);
const mixed={name:'Mixed',graphs:[{name:'Vertical',config:current.config({mode:'bezier',curve:[0,.4,1,.6]})},{name:'Spring',config:current.config({mode:'spring'})},{name:'Quart',config:current.config({mode:'standard',family:'quart'})}]};
const mixedFlow=current.flowExport(mixed);
assert.deepEqual(mixedFlow.skipped,['Spring','Quart']);
assert.deepEqual(JSON.parse(mixedFlow.text)[0].value,[0,.4,1,.6]);
assert.throws(()=>current.flowExport({name:'Motion',graphs:[mixed.graphs[1]]}),/no Bezier/);
assert.deepEqual(current.decodeCurrent(current.encodeCurrent({format:'current.graph-lists',version:1,lists:[mixed]})).lists[0],mixed);
p=new Property();h=host(p);
const vertical=mixed.graphs[0].config;
assert.equal(current.apply(h,vertical).approximated,true);
assert.deepEqual(vertical.curve,[0,.4,1,.6], 'applying must preserve saved control points');
console.log('Current portable math, libraries, Flow interoperability and public-API model checks passed. Actual After Effects runtime remains unverified.');
