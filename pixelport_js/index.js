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
var reconnect = require('reconnect-core')(function () {
  return net.connect.apply(null, arguments);
});
var ponParse = require('./pon');


class Pixelport extends EventEmitter {
  constructor() {
    super();
    this.client = null;
    this._writeStream = null;
    this.process = null;
    this.channels = {};
    this.channelCounter = 1;
    this.on('newListener', (event) => {
      if (event == 'frame' && !this.frameStream) {
        this.frameStream = this.stream('frame_stream_create');
        this.frameStream.on('message', (frame) => this.emit('frame', frame));
      }
    });
    this.on('removeListener', (event) => {
      if (event == 'frame' && this.frameStream && this.listenerCount('frame') == 0) {
        this.frameStream.destroy();
        delete this.frameStream;
      }
    });
  }

  request(message) {
    var channelId = this.channelCounter++;
    message = message.replace(/\n/g, '');
    return new Promise((resolve, reject) => {
      debug_request("id=%d, %o", channelId, message);
      this.channels[channelId] = (status, body) => {
        if (status == 'ok') resolve(body);
        else reject(new Promise.OperationalError(body));
        delete this.channels[channelId];
      };
      this._writeStream.write(channelId + ' ' + message + '\n');
    });
  }

  stream(message) {
    var channelId = this.channelCounter++;
    let stream = new Stream(this, channelId);
    this.channels[channelId] = (status, body) => {
      if (status == 'ok') stream.emit('message', body);
      else {
        stream.emit('error', body);
        delete this.channels[channelId];
      }
    };
    this._writeStream.write(channelId + ' ' + message + '\n');
    return stream;
  }

  closeStream(channelId) {
    return this.request(`close_stream { channel_id: '${channelId}' }`).then(() => {
      delete this.channels[channelId];
    });
  }

  shutdown() {
    this.process.kill();
  }

  // Helpers
  waitForEntity(selector) {
    return new Promise((resolve, reject) => {
      let stream = this.stream(`doc_stream_create { selector: ${selector} }`);
      stream.on('message', (changes) => {
        if (changes.arg.entities_added.length > 0) {
          stream.destroy();
          resolve();
        }
      });
    });
  }

  waitForPropertyChange(selector, property) {
    return new Promise((resolve, reject) => {
      let stream = this.subDocStreamCreate({ selector: selector, property_regex: property });
      stream.on('message', (changes) => {
        if (changes.arg.updated_properties.length > 0) {
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
    return this.request(`fake_window_event window_event_mouse_moved { x: ${position.x}, y: ${position.y} }`);
  }

  fakeClick() {
    return this.request(`fake_window_event window_event_mouse_input { state: 'pressed', button: 'left' }`);
  }

  _handleMessage(message) {
    debug_response("%s", message);
    message = message.split(' ');
    let channelId = message[0];
    let status = message[1];
    let bodyString = message.slice(2).join(' ');
    let body = Pixelport.parsePon(bodyString);
    var channel = this.channels[channelId];
    if (channel) {
      channel(status, body);
    }
  }

  connectToWindow(opts) {
    return new Promise((resolve, reject) => {
      opts = opts || {};
      let address = opts.address || {};
      address.port = address.port || 4303;
      this.connection = {
        address: address
      };
      let reconnectOpts = opts.reconnect || {}; // See https://www.npmjs.com/package/reconnect-core for options
      if (reconnectOpts === true) { reconnectOpts = {}; }
      debug('Connecting to Pixelport %o', opts);
      var streamPromiseResolve = null;
      var streamPromise = new Promise((resolve) => streamPromiseResolve = resolve );
      this.client = reconnect(reconnectOpts, (stream) => {
        this._writeStream = stream;
        //stream.readable = true; // Just to get byline working
        var lines = byline(stream);

        lines.on('data', line => {
          this._handleMessage(line.toString());
        });
        streamPromiseResolve();
      })
      .on('connect', (con) => {
        // Make sure the stream is created first
        streamPromise.then(() => {
          debug('Connected to %o', address);
          resolve();
          this.emit('connect');
        });
      })
      .on('reconnect', (n, delay) => {
        debug('Reconnected to %o', address);
        this.emit('reconnect');
      })
      .on('disconnect', (err) => {
        debug('Disconnected from %o', address);
        this.emit('disconnect');
      })
      .on('error', (err) => {
        debug("Socket error: %o", err);
        this.emit('socket-error', err);
      })
      .connect(address);
      this.client.reconnect = !!opts.reconnect;
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
      return this.connectToWindow({ address: { port: config.port }, reconnect: opts.reconnect });
    })
  }

  createOrConnectToWindow(opts) {
    if (process.env.PIXELPORT_CONNECT_TO) {
      opts = opts || {};
      opts.address = opts.address || {};
      opts.address.port = parseInt(process.env.PIXELPORT_CONNECT_TO);
      return this.connectToWindow(opts);
    } else {
      return this.createWindow(opts);
    }
  }

  static ponEscape(str) {
    return str.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
  }
  static ponUnescape(str) {
    return str.replace(/\\\\/g, "\\").replace(/\\'/g, "'");
  }
  static parsePon(str) {
    return ponParse.parse(str);
  }
  static stringifyPon(pon) {
    if (pon._transform) {
      return pon._transform + ' ' + Pixelport.stringifyPon(pon.arg);
    } else if (Array.isArray(pon)) {
      return '[ ' + pon.map(x => Pixelport.stringifyPon(x)).join(', ') + ' ]';
    } else if (pon instanceof Object) {
      return '{ ' + Object.keys(pon).map(k => k + ': ' + Pixelport.stringifyPon(pon[k])).join(', ') + ' }';
    } else {
      return pon;
    }
  }
  static stringifyVec3(v) {
    return Pixelport.stringifyPon({ _transform: 'vec3', arg: v });
  }
  static parseVec3(str) {
    let vec3 = Pixelport.parsePon(str).arg;
    vec3.x = vec3.x || 0;
    vec3.y = vec3.y || 0;
    vec3.z = vec3.z || 0;
    return vec3;
  }
  static colorToString(v) {
    return Pixelport.stringifyPon({ _transform: 'color', arg: v });
  }
}

module.exports = Pixelport;

class Stream extends EventEmitter {
  constructor(pixelport, id) {
    super();
    this.pixelport = pixelport;
    this.id = id;
  }
  destroy() {
    return this.pixelport.closeStream(this.id);
  }
}
