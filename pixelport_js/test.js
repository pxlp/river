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
