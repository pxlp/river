"use strict";

var assert = require('chai').assert;
var Pixelport = require('.');

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
    assert.deepEqual(Pixelport.parsePon("vec3 {}"), {  _transform: 'vec3', arg: {} });
  });

  test('just x', function() {
    assert.deepEqual(Pixelport.parsePon("vec3 { x: 1 }"), { _transform: 'vec3', arg: { x: 1 } });
  });

  test('full', function() {
    assert.deepEqual(Pixelport.parsePon("vec3 { x: -1.56, y: 33, z: 533.12 }"), { _transform: 'vec3', arg: { x: -1.56, y: 33, z: 533.12 } });
  });

  test('spaces', function() {
    assert.deepEqual(Pixelport.parsePon("vec3{   x  :   -1.56  ,y:33, z:   533.12}"), { _transform: 'vec3', arg: { x: -1.56, y: 33, z: 533.12 } });
  });

  test('string', function() {
    assert.deepEqual(Pixelport.parsePon("test { x: 'hello' }"), { _transform: 'test', arg: { x: "'hello'" } });
  });

});

suite('pon stringify', function() {

  test('string', function() {
    // Strings are assumed to be pre-transformed pons.
    assert.deepEqual(Pixelport.stringifyPon("test"), "test");
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
