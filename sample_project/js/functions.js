// Property identifier in a 'pair' with a 'function_expression'
const objWithFuncExpr = {
  propFuncExpr: function () {
    console.log("Hello, world!");
  },
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

// Assignment to module.exports with a named function expression
module.exports.exampleFunc = function exampleFuncInner() {
  console.log("hello");
};

// Assignment to module.exports with an arrow function
module.exports.exampleFunc = () => {
  console.log("hello");
}
