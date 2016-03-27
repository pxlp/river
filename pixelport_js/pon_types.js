"use strict";

class PonCall {
  constructor(functionName, arg) {
    this.functionName = functionName;
    this.arg = arg;
  }
}

class PonPropRef {
  constructor(propref) {
    this.propref = propref;
  }
}

class PonDepPropRef {
  constructor(propref) {
    this.propref = propref;
  }
}

class PonSelector {
  constructor(selector) {
    this.selector = selector;
  }
}


module.exports.PonCall = PonCall;
module.exports.PonPropRef = PonPropRef;
module.exports.PonDepPropRef = PonDepPropRef;
module.exports.PonSelector = PonSelector;
