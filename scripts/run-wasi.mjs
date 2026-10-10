// Runs a wasm32-wasip1 binary under Node's built-in WASI runtime.
// Usage: node scripts/run-wasi.mjs path/to/program.wasm [args...]
import { readFile } from 'node:fs/promises';
import { WASI } from 'node:wasi';
import process from 'node:process';

const [path, ...args] = process.argv.slice(2);
if (!path) {
  console.error('usage: node scripts/run-wasi.mjs <program.wasm> [args...]');
  process.exit(2);
}
const wasi = new WASI({ version: 'preview1', args: [path, ...args], env: {}, returnOnExit: true });
const module = await WebAssembly.compile(await readFile(path));
const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
process.exitCode = wasi.start(instance);
