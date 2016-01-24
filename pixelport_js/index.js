"use strict";

var child_process = require('child_process');
var net = require('net');
var Promise = require("bluebird");
var byline = require('byline');
var EventEmitter = require('events');
var util = require('util');
var debug = require('debug')('pixelport');
var debug_actions = require('debug')('pixelport:actions');
var debug_window_stdout = require('debug')('pixelport:window:stdout');
var debug_window_stderr = require('debug')('pixelport:window:stderr');

class Pyramid extends EventEmitter {
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
      this.pending[requestId] = { resolve: resolve, reject: reject };
      this.client.write(JSON.stringify(cmd) + '\r\n');
    });
  }

  setProperties(entitySelector, properties) {
    debug_actions('setProperties for %s: %o', entitySelector, properties);
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
    debug_actions('appendEntity parent: %s, type name: %s, properties: %o', parentSelector, typeName, properties);
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

  subDocStreamCreate(opts) {
    opts.id = opts.id || ('subdocstream-' + this.subdocStreamIdCounter++);
    debug_actions('subDocStreamCreate %o', opts);
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
    debug_actions('subDocStreamDestroy id: %s', id);
    return this._request({
      SubDocStreamDestroy: {
        id: id
      }
    });
  }

  screenshot() {
    debug_actions('screenshot');
    return this._request({
      Screenshot: []
    }).then(function(resp) {
      return resp.Screenshot;
    });
  }

  viewportRebuildScene() {
    debug_actions('viewportRebuildScene');
    return this._request({
      ViewportRebuildScene: []
    });
  }

  viewportUpdateAllUniforms() {
    debug_actions('viewportUpdateAllUniforms');
    return this._request({
      ViewportUpdateAllUniforms: []
    });
  }

  viewportDumpPipelines() {
    debug_actions('viewportDumpPipelines');
    return this._request({
      ViewportDumpPipelines: []
    });
  }

  viewportDumpResources() {
    debug_actions('viewportDumpResources');
    return this._request({
      ViewportDumpResources: []
    });
  }


  entityRenderersBounding(entitySelector) {
    debug_actions('entityRenderersBounding');
    return this._request({
      EntityRenderersBounding: {
        entity_selector: entitySelector
      }
    }).then(function(resp) {
      return resp.EntityRenderersBounding;
    });
  }

  visualizeEntityRenderersBounding(entitySelector) {
    debug_actions('entityRenderersBounding');
    return this._request({
      VisualizeEntityRenderersBounding: {
        entity_selector: entitySelector
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

  _handleMessage(message) {
    if (message.Frame) {
      this.emit('frame', message.Frame);
    } else if (message.Response) {
      var pending = this.pending[message.Response.request_id];
      if (pending) {
        delete this.pending[message.Response.request_id];
        if (message.Response.response.Ok)
          pending.resolve(message.Response.response.Ok.data);
        else
          pending.reject(message.Response.response.Fail.error);
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
      address.port = address.port || 8081;
      debug('Connecting to window on address %o', address);
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
        this.emit('closed');
      });
    });
  }

  createWindow(opts) {
    opts = opts || {};
    opts.port = opts.port || 8081;
    opts.pixelportAppPath = opts.pixelportAppPath || process.env.PYRAMID_APP_PATH || '../../pixelport_app/target/release/pixelport_app';

    var startArgs = opts.startArgs = opts.startArgs || [];
    startArgs.push('--port=' + opts.port);
    if (opts.startPml) {
      startArgs.push(opts.startPml);
    }
    debug('Creating pixelport window with opts: %o', opts);

    var createPromise = new Promise((resolve, reject) => {
      this.process = child_process.spawn(opts.pixelportAppPath, startArgs, { env: { "RUST_BACKTRACE": 1, "RUST_LOG": "info" } });

      byline(this.process.stdout).on('data', function (line) {
        line = line.toString();
        if (line.indexOf("## READY FOR CONNECTIONS ##") >= 0) {
          resolve();
        } else {
          debug_window_stdout(line);
        }
      });

      byline(this.process.stderr).on('data', function (line) {
        line = line.toString();
        debug_window_stderr(line);
      });

      this.process.on('close', function (code) {
        debug('Pyramid exited with code: %s', code);
      });
    });

    return createPromise.then(() => {
      return this.connectToWindow({ port: opts.port });
    })
  }

  createOrConnectToWindow(opts) {
    if (process.env.PYRAMID_CONNECT_TO) {
      return this.connectToWindow({
        port: parseInt(process.env.PYRAMID_CONNECT_TO)
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

module.exports = Pyramid;

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
