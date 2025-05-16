// Property identifier in a 'pair' with a 'function_expression'
const objWithFuncExpr = {
  propFuncExpr: function () {
    console.log("Hello, world!");
  },
};

// Property identifier in a 'method_definition' (class method)
class MyClassExample {
  classMethodRegular() {}

  static staticClassMethod() {}

  get getterMethod() {
    return this._x;
  }

  set setterMethod(value) {
    this._x = value;
  }
}

// Property identifier in a 'method_definition' (object shorthand method)
const objWithShorthand = {
  shorthandObjMethod() {},

  *generatorShorthandMethod() {},

  async asyncShorthandMethod() {},
};

// Property identifier in a 'pair' with an 'arrow_function'
const objWithArrowFunc = {
  propArrowFunc: () => {},
};

// Top-level function declaration
function topLevelStandardFunction() {}

// Variable declarator with an arrow function
const topLevelArrowConst = () => {};

// Variable declarator with a function expression
const topLevelFuncExprConst = function namedInnerFuncExpr() {};

// Assignment expression with an arrow function
let assignedArrowLet;
assignedArrowLet = () => {};
