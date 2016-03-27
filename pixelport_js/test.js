"use strict";

var assert = require('chai').assert;
var Pixelport = require('.');
var pon = Pixelport.ponConstruct;
var PonCall = require('./pon_types').PonCall;
var PonDepPropRef = require('./pon_types').PonDepPropRef;
var PonPropRef = require('./pon_types').PonPropRef;
var PonSelector = require('./pon_types').PonSelector;

suite('request', function() {

  test('requests returned promise should resolve', function(done) {
    var pixelport = new Pixelport();
    pixelport.createWindow({
      port: 0,
      args: ['--multisampling=0', '--headless']
    }).then(function() {
      pixelport.request(`set_properties { entity: root, properties: { x: 5 } }`).then(function() {
        done();
      });
    });
  });

});


suite('stream', function() {

  test('doc stream', function(done) {
    var pixelport = new Pixelport();
    pixelport.createWindow({
      port: 0,
      args: ['--multisampling=0', '--headless']
    }).then(function() {
      let stream = pixelport.stream(`doc_stream_create { selector: root, properties: '.*' }`);
      stream.on('message', () => {
        done()
      });
      pixelport.request(`set_properties { entity: root, properties: { x: 5 } }`);
    });
  });

});


suite('pon parsing', function() {

  test('empty', function() {
    assert.deepEqual(Pixelport.parsePon("vec3 {}"), new PonCall("vec3", {}));
  });

  test('just x', function() {
    assert.deepEqual(Pixelport.parsePon("vec3 { x: 1 }"), new PonCall("vec3", { x: 1 }));
  });

  test('full', function() {
    assert.deepEqual(Pixelport.parsePon("vec3 { x: -1.56, y: 33, z: 533.12 }"), new PonCall("vec3", { x: -1.56, y: 33, z: 533.12 }));
  });

  test('spaces', function() {
    assert.deepEqual(Pixelport.parsePon("vec3{   x  :   -1.56  ,y:33, z:   533.12}"), new PonCall("vec3", { x: -1.56, y: 33, z: 533.12 }));
  });

  test('string', function() {
    assert.deepEqual(Pixelport.parsePon("test { x: 'hello' }"), new PonCall("test", { x: "hello" }));
  });

  test('selector', function() {
    assert.deepEqual(Pixelport.parsePon("test { x: #15 }"), new PonCall("test", { x: new PonSelector("#15") }));
  });

  test('propref', function() {
    assert.deepEqual(Pixelport.parsePon("test { x: root:Hello.y }"), new PonCall("test", { x: new PonPropRef("root:Hello.y") }));
  });

  test('dep_propref', function() {
    assert.deepEqual(Pixelport.parsePon("test { x: @root:Hello.y }"), new PonCall("test", { x: new PonDepPropRef("root:Hello.y") }));
  });

  test('array', function() {
    assert.deepEqual(Pixelport.parsePon("test { x: [5, 3] }"), new PonCall("test", { x: [5, 3] }));
  });

  test('nil', function() {
    assert.deepEqual(Pixelport.parsePon("test { x: () }"), new PonCall("test", { x: null }));
  });

  test('boolean', function() {
    assert.deepEqual(Pixelport.parsePon("test { x: true }"), new PonCall("test", { x: true }));
  });
});

suite('pon stringify', function() {

  test('string', function() {
    assert.deepEqual(Pixelport.stringifyPon("test"), "'test'");
  });

  test('propref', function() {
    assert.deepEqual(Pixelport.stringifyPon(new PonCall("test", { x: new PonPropRef("#15.5") })), "test { x: #15.5 }");
  });

  test('dep_propref', function() {
    assert.deepEqual(Pixelport.stringifyPon(new PonCall("test", { x: new PonDepPropRef("#15.5") })), "test { x: @#15.5 }");
  });

  test('selector', function() {
    assert.deepEqual(Pixelport.stringifyPon(new PonCall("test", { x: new PonSelector("#15") })), "test { x: #15 }");
  });

});

suite('vec3 parsing', function() {

  test('empty', function() {
    assert.deepEqual(Pixelport.parseVec3("vec3 {}"), { x: 0, y: 0, z: 0 });
  });

  test('just x', function() {
    assert.deepEqual(Pixelport.parseVec3("vec3 { x: 1 }"), { x: 1, y: 0, z: 0 });
  });

  test('full', function() {
    assert.deepEqual(Pixelport.parseVec3("vec3 { x: -1.56, y: 33, z: 533.12 }"), { x: -1.56, y: 33, z: 533.12 });
  });

  test('spaces', function() {
    assert.deepEqual(Pixelport.parseVec3("vec3{   x  :   -1.56  ,y:33, z:   533.12}"), { x: -1.56, y: 33, z: 533.12 });
  });

});
