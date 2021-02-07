const { promisify } = require("util");

try {
  const native = require("./index.node");
  const app_command_promise = promisify(native.app_command);

  module.exports = { ...native, app_command_promise };
} catch (e) {
  console.error(e);
}
