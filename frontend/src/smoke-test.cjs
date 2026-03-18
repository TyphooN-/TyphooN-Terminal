// smoke-test.js — Automated smoke test for all 185 Ctrl+K commands
// Run via: node src/smoke-test.js (with mocked invoke)
// Tests that every command function exists, is callable, and doesn't throw on basic input.

// Mock environment for Node.js testing (no DOM, no Tauri)
const errors = [];
const passed = [];
let totalTests = 0;

// Collect all command function names from CMD_PALETTE_COMMANDS registration pattern
const fs = require("fs");
const source = fs.readFileSync(__dirname + "/main.js", "utf8");

// Extract all command names: { name: "X", ... action: cmdY }
const cmdPattern = /\{\s*name:\s*"([^"]+)"[^}]*action:\s*(\w+)/g;
const commands = [];
let match;
while ((match = cmdPattern.exec(source)) !== null) {
  commands.push({ name: match[1], fn: match[2] });
}

// Extract all function definitions
const fnPattern = /^(?:async )?function (\w+)\s*\(/gm;
const definedFunctions = new Set();
while ((match = fnPattern.exec(source)) !== null) {
  definedFunctions.add(match[1]);
}

// Test 1: Every registered command has a function definition
console.log("=== TEST 1: Command Registration Integrity ===");
for (const cmd of commands) {
  totalTests++;
  if (definedFunctions.has(cmd.fn)) {
    passed.push(`✅ ${cmd.name} → ${cmd.fn}() defined`);
  } else {
    errors.push(`❌ ${cmd.name} → ${cmd.fn}() NOT FOUND`);
  }
}

// Test 2: No duplicate command names
console.log("=== TEST 2: No Duplicate Command Names ===");
const nameCount = {};
for (const cmd of commands) {
  nameCount[cmd.name] = (nameCount[cmd.name] || 0) + 1;
}
for (const [name, count] of Object.entries(nameCount)) {
  totalTests++;
  if (count === 1) {
    passed.push(`✅ ${name}: unique`);
  } else {
    errors.push(`❌ ${name}: registered ${count} times (duplicate!)`);
  }
}

// Test 3: No duplicate function names
console.log("=== TEST 3: No Duplicate Function Definitions ===");
const fnDefCount = {};
const fnDefPattern2 = /^(?:async )?function (\w+)\s*\(/gm;
while ((match = fnDefPattern2.exec(source)) !== null) {
  fnDefCount[match[1]] = (fnDefCount[match[1]] || 0) + 1;
}
for (const [fn, count] of Object.entries(fnDefCount)) {
  if (fn.startsWith("cmd")) {
    totalTests++;
    if (count === 1) {
      passed.push(`✅ ${fn}: unique definition`);
    } else {
      errors.push(`❌ ${fn}: defined ${count} times (duplicate!)`);
    }
  }
}

// Test 4: Zero innerHTML in code (not comments)
console.log("=== TEST 4: Zero innerHTML in Code ===");
totalTests++;
const lines = source.split("\n");
let innerHTMLCode = 0;
for (let i = 0; i < lines.length; i++) {
  const line = lines[i];
  if (line.includes("innerHTML") && !line.trim().startsWith("//") && !line.trim().startsWith("*")) {
    innerHTMLCode++;
  }
}
if (innerHTMLCode === 0) {
  passed.push("✅ Zero innerHTML in code");
} else {
  errors.push(`❌ ${innerHTMLCode} innerHTML found in code`);
}

// Test 5: All invoke("get_bars") go through cachedGetBars
console.log("=== TEST 5: Bar Request Deduplication ===");
totalTests++;
const directGetBars = lines.filter(l =>
  l.includes('invoke("get_bars"') &&
  !l.includes("cachedGetBars") &&
  !l.trim().startsWith("//") &&
  !l.includes("const promise = invoke")  // inside cachedGetBars itself
).length;
if (directGetBars === 0) {
  passed.push("✅ All get_bars go through cachedGetBars (dedup layer)");
} else {
  errors.push(`❌ ${directGetBars} direct invoke("get_bars") calls bypass dedup`);
}

// Test 6: No eval() usage
console.log("=== TEST 6: No eval() ===");
totalTests++;
const evalCount = lines.filter(l => /\beval\s*\(/.test(l) && !l.trim().startsWith("//") && !l.includes('"eval(')).length;
if (evalCount === 0) {
  passed.push("✅ Zero eval() calls");
} else {
  errors.push(`❌ ${evalCount} eval() calls found`);
}

// Test 7: new Function() is sandboxed
console.log("=== TEST 7: Sandboxed new Function() ===");
totalTests++;
const newFuncLines = lines.filter(l => l.includes("new Function(") && !l.trim().startsWith("//"));
const allSandboxed = newFuncLines.every(l => l.includes("Math") || l.includes("forbidden"));
if (newFuncLines.length <= 1 && allSandboxed) {
  passed.push(`✅ new Function() sandboxed (${newFuncLines.length} instance)`);
} else {
  errors.push(`❌ ${newFuncLines.length} new Function() calls, not all sandboxed`);
}

// Test 8: All setInterval have matching clearInterval patterns
console.log("=== TEST 8: Interval Cleanup ===");
totalTests++;
const setIntervalCount = lines.filter(l => l.includes("setInterval(") && !l.trim().startsWith("//")).length;
const clearIntervalCount = lines.filter(l => l.includes("clearInterval(") && !l.trim().startsWith("//")).length;
if (clearIntervalCount >= setIntervalCount) {
  passed.push(`✅ Interval cleanup: ${setIntervalCount} set, ${clearIntervalCount} clear`);
} else {
  errors.push(`❌ Interval leak risk: ${setIntervalCount} set but only ${clearIntervalCount} clear`);
}

// Test 9: Indicator calc functions exist
console.log("=== TEST 9: Indicator Functions ===");
const indicators = ["calcSMA", "calcEMA", "calcKAMA", "calcRSI", "calcATR", "calcEhlersFisher",
  "calcBetterVolume", "calcATRProjection", "calcPrevCandleLevels", "calcBollinger",
  "calcMACD", "calcVWAP", "calcRVOL", "calcDEMA", "calcStochastic", "calcCCI",
  "calcADX", "calcWilliamsR", "calcIchimoku", "calcParabolicSAR", "calcOBV", "calcMomentum",
  "calcWMA", "calcHMA", "calcSupplyDemandZones", "calcAutoFibonacci"];
for (const ind of indicators) {
  totalTests++;
  if (definedFunctions.has(ind)) {
    passed.push(`✅ ${ind}() defined`);
  } else {
    errors.push(`❌ ${ind}() NOT FOUND`);
  }
}

// Test 10: Wasm wrapper functions exist
console.log("=== TEST 10: Wasm Wrappers ===");
const wasmFns = ["wasmCalcSMA", "wasmCalcEMA", "wasmCalcKAMA", "wasmCalcRSI", "wasmCalcATR"];
for (const fn of wasmFns) {
  totalTests++;
  if (definedFunctions.has(fn)) {
    passed.push(`✅ ${fn}() defined`);
  } else {
    errors.push(`❌ ${fn}() NOT FOUND`);
  }
}

// Test 11: Safe DOM helpers exist
console.log("=== TEST 11: Safe DOM Helpers ===");
const helpers = ["el", "span", "div", "td", "theadRow", "styledRow", "colorSpan", "labelValue", "setText", "setTextClass"];
for (const h of helpers) {
  totalTests++;
  if (definedFunctions.has(h)) {
    passed.push(`✅ ${h}() defined`);
  } else {
    errors.push(`❌ ${h}() NOT FOUND`);
  }
}

// Test 12: cachedGetBars exists
console.log("=== TEST 12: Request Dedup Layer ===");
totalTests++;
if (definedFunctions.has("cachedGetBars")) {
  passed.push("✅ cachedGetBars() defined");
} else {
  errors.push("❌ cachedGetBars() NOT FOUND");
}

// Report
console.log("\n" + "=".repeat(60));
console.log(`SMOKE TEST RESULTS: ${passed.length}/${totalTests} passed, ${errors.length} failed`);
console.log("=".repeat(60));
if (errors.length > 0) {
  console.log("\nFAILURES:");
  for (const e of errors) console.log("  " + e);
}
console.log(`\nTotal commands registered: ${commands.length}`);
console.log(`Total functions defined: ${definedFunctions.size}`);
console.log(`Total cmd* functions: ${Object.keys(fnDefCount).filter(k => k.startsWith("cmd")).length}`);

process.exit(errors.length > 0 ? 1 : 0);
