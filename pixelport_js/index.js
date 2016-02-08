"use strict";

var child_process = require('child_process');
var net = require('net');
var Promise = require("bluebird");
var byline = require('byline');
var EventEmitter = require('events');
var util = require('util');
var debug = require('debug')('pixelport');
var debug_request = require('debug')('pixelport:request');
var debug_response = require('debug')('pixelport:response');
var debug_window_stdout = require('debug')('pixelport:window:stdout');
var debug_window_stderr = require('debug')('pixelport:window:stderr');

class Pixelport extends EventEmitter {
  constructor() {
    super();
    this.client = null;
    this.process = null;
    this.pending = {};
    this.rpcIdCounter = 1;
    this.subdocStreamIdCounter = 1;
    this.subDocStreams = {};
  }
  static ponEscape(str) {
    return str.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
  }
  static ponUnescape(str) {
    return str.replace(/\\\\/g, "\\").replace(/\\'/g, "'");
  }

  _request(message) {
    var requestId = this.rpcIdCounter++;
    return new Promise((resolve, reject) => {
      var cmd = {
        Request: {
          request_id: requestId,
          request: message
        }
      };
      debug_request("id=%d, %o", requestId, message);
      this.pending[requestId] = { resolve: resolve, reject: reject };
      this.client.write(JSON.stringify(cmd) + '\r\n');
    });
  }

  setProperties(entitySelector, properties) {
    Object.keys(properties).forEach(function(key) {
      properties[key] = '' + properties[key]; // Make sure properties are strings
    });
    return this._request({
      SetProperties: {
        entity_selector: '' + entitySelector,
        properties: properties
      }
    });
  }

  appendEntity(parentSelector, typeName, properties) {
    properties = properties || {};
    Object.keys(properties).forEach(function(key) {
      properties[key] = '' + properties[key]; // Make sure properties are strings
    });
    return this._request({
      AppendEntity: {
        parent_selector: parentSelector,
        type_name: typeName,
        properties: properties
      }
    }).then(function(resp) {
      return resp.EntityAdded;
    });
  }

  removeEntity(entitySelector) {
    return this._request({
      RemoveEntity: {
        entity_selector: entitySelector
      }
    });
  }

  clearChildren(entitySelector) {
    return this._request({
      ClearChildren: {
        entity_selector: entitySelector
      }
    });
  }

  subDocStreamCreate(opts) {
    opts.id = opts.id || ('subdocstream-' + this.subdocStreamIdCounter++);
    var subDocStream = new SubDocStream(this, opts.id);
    this.subDocStreams[opts.id] = subDocStream;
    this._request({
      SubDocStreamCreate: {
        id: opts.id,
        selector: opts.selector,
        property_regex: opts.property_regex || '.*',
        include_invalidated: !!opts.include_invalidated
      }
    });
    return subDocStream;
  }

  subDocStreamDestroy(id) {
    return this._request({
      SubDocStreamDestroy: {
        id: id
      }
    });
  }

  screenshot() {
    return this._request({
      Screenshot: []
    }).then(function(resp) {
      return resp.Screenshot;
    });
  }

  screenshotToFile(path) {
    return this._request({
      ScreenshotToFile: { path: path }
    });
  }

  pause() {
    return this._request({
      Pause: []
    });
  }

  cont() {
    return this._request({
      Continue: []
    });
  }

  step() {
    return this._request({
      Step: []
    });
  }

  viewportRebuildScene() {
    return this._request({
      ViewportRebuildScene: []
    });
  }

  viewportUpdateAllUniforms() {
    return this._request({
      ViewportUpdateAllUniforms: []
    });
  }

  viewportDumpPipelines() {
    return this._request({
      ViewportDumpPipelines: []
    });
  }

  viewportDumpResources() {
    return this._request({
      ViewportDumpResources: []
    });
  }


  entityRenderersBounding(entitySelector) {
    return this._request({
      EntityRenderersBounding: {
        entity_selector: entitySelector
      }
    }).then(function(resp) {
      return resp.EntityRenderersBounding;
    });
  }

  visualizeEntityRenderersBounding(entitySelector) {
    return this._request({
      VisualizeEntityRenderersBounding: {
        entity_selector: entitySelector
      }
    });
  }

  fakeWindowEvent(event) {
    return this._request({
      FakeWindowEvent: {
        event: event
      }
    });
  }

  shutdown() {
    this.process.kill();
  }

  // Helpers
  waitForEntity(selector) {
    return new Promise((resolve, reject) => {
      let stream = this.subDocStreamCreate({ selector: selector });
      stream.on('cycle', (changes) => {
        if (changes.entities_added.length > 0) {
          stream.destroy();
          resolve();
        }
      });
    });
  }

  waitForPropertyChange(selector, property) {
    return new Promise((resolve, reject) => {
      let stream = this.subDocStreamCreate({ selector: selector, property_regex: property });
      stream.on('cycle', (changes) => {
        if (changes.set_properties.length > 0) {
          stream.destroy();
          resolve();
        }
      });
    });
  }

  waitFrames(n) {
    if (n === undefined) n = 1;
    return new Promise((resolve, reject) => {
      let cb = () => {
        n--;
        if (n == 0) {
          this.removeListener('frame', cb);
          resolve();
        }
      };
      this.on('frame', cb);
    });
  }

  fakeMoveMouse(position) {
    return this.fakeWindowEvent({ MouseMoved: [position.x, position.y] });
  }

  fakeClick() {
    return this.fakeWindowEvent({ MouseInput: { state: { Pressed: [] }, button: { Left: [] } } });
  }

  _handleMessage(message) {
    if (message.Frame) {
      this.emit('frame', message.Frame);
    } else if (message.Response) {
      var pending = this.pending[message.Response.request_id];
      if (pending) {
        delete this.pending[message.Response.request_id];
        if (message.Response.response.Ok) {
          if (message.Response.response.Ok.data.Screenshot) { // Screenshots are massive, hiding it from the console
            debug_response("id=%d, OK Screenshot", message.Response.request_id);
          } else {
            debug_response("id=%d, OK %o", message.Response.request_id, message.Response.response.Ok.data);
          }
          pending.resolve(message.Response.response.Ok.data);
        } else {
          debug_response("id=%d, FAIL %o", message.Response.request_id, message.Response.response.Fail.error);
          pending.reject(message.Response.response.Fail.error);
        }
      }
    } else if (message.SubDocStreamCycle) {
      let cycle = message.SubDocStreamCycle;
      var subDocStream = this.subDocStreams[cycle.sub_doc_stream_id];
      subDocStream.emit('cycle', cycle);
    }
  }

  connectToWindow(address) {
    return new Promise((resolve, reject) => {
      address = address || {};
      address.port = address.port || 4303;
      debug('Connecting to window on address %o', address);
      this.connection = {
        address: address
      };
      this.client = net.connect(address, function() {
        debug('Connected to pixelport app!');
        resolve();
      });
      this.client.readable = true; // Just to get byline working
      var lines = byline(this.client);

      this.client.on('error', function(error) {
        debug("Socket error: %o", error);
        reject(error);
      });

      lines.on('data', line => {
        var message = JSON.parse(line);
        this._handleMessage(message);
      });

      this.client.on('end', () => {
        debug('Connection to %o ended', address);
        this.emit('closed');
      });
    });
  }

  createWindow(opts) {
    opts = opts || {};
    opts.port = opts.port !== undefined ? opts.port : 4303;
    opts.pixelportAppPath = opts.pixelportAppPath || process.env.PIXELPORT_APP_PATH;
    if (!opts.pixelportAppPath) {
      throw new Error(`Pixelport app path not specified. Set the environment variable
PIXELPORT_APP_PATH to the full path of the pixelport app. For instance:
$ export PIXELPORT_APP_PATH=~/pixelport/pixelport_app/target/release/pixelport_app`);
    }

    var args = opts.args = opts.args || [];
    args.push('--port=' + opts.port);
    if (opts.document) {
      args.push(opts.document);
    }
    debug('Creating pixelport window with opts: %o', opts);

    var createPromise = new Promise((resolve, reject) => {
      this.process = child_process.spawn(opts.pixelportAppPath, args, { env: { "RUST_BACKTRACE": 1, "RUST_LOG": opts.log || "info" } });

      let readConfigNext = false;
      byline(this.process.stdout).on('data', function (line) {
        line = line.toString();
        if (line.indexOf("## READY FOR CONNECTIONS ##") >= 0) {
          readConfigNext = true;
        } else if (readConfigNext) {
          readConfigNext = false;
          let config = JSON.parse(line);
          resolve(config);
        } else {
          debug_window_stdout(line);
        }
      });

      byline(this.process.stderr).on('data', function (line) {
        line = line.toString();
        debug_window_stderr(line);
      });

      this.process.on('close', function (code) {
        debug('Pixelport exited with code: %s', code);
      });
    });

    return createPromise.then((config) => {
      this.window = {
        port: config.port
      };
      return this.connectToWindow({ port: config.port });
    })
  }

  createOrConnectToWindow(opts) {
    if (process.env.PIXELPORT_CONNECT_TO) {
      return this.connectToWindow({
        port: parseInt(process.env.PIXELPORT_CONNECT_TO)
      });
    } else {
      return this.createWindow(opts);
    }
  }

  static vec3ToString(v) {
    return 'vec3 { x: ' + (v.x || 0.0) + ', y: ' + (v.y || 0.0) + ', z: ' + (v.z || 0) + '}';
  }
  static colorToString(v) {
    return 'color { r: ' + (v.r || 0.0) + ', g: ' + (v.g || 0.0) + ', b: ' + (v.b || 0) + ', a: ' + (v.a || 1) + '}';
  }
}

module.exports = Pixelport;

class SubDocStream extends EventEmitter {
  constructor(pixelport, id) {
    super();
    this.pixelport = pixelport;
    this.id = id;
  }
  destroy() {
    return this.pixelport.subDocStreamDestroy(this.id);
  }
}
