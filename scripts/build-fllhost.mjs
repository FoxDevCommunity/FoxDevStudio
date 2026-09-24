// Builds the 32-bit process that hosts Visual FoxPro libraries, and - when this machine has
// Visual FoxPro's own API samples - the two sample libraries the tests measure the host against.
//
// It is x86 and it is a process of its own because a .fll is a 32-bit image and Node is not:
// nothing loaded into the application itself could ever open one. Windows only, and only where
// Visual C++ is installed; everywhere else `SET LIBRARY TO` says it cannot host a library, which
// is the honest answer rather than a silent one.
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, rmSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { msvcEnv } from './msvc.mjs';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
// Shipped the same way foxole.node is: electron-builder carries resources/** and unpacks
// resources/native/** out of the asar, where a real file has to be. A folder of its own,
// because the Visual C++ 7.1 runtimes go in it beside the host - see the README written there.
const outDir = join(root, 'resources', 'native', 'win32');
// the sample libraries are test fixtures, not part of the product, so they are built where
// nothing packages them
const sampleDir = join(root, 'tests', 'fll', 'build');
const work = join(root, 'resources', 'native', 'obj');

if (process.platform !== 'win32') {
  console.log('[build-fllhost] not Windows: there are no FoxPro libraries to host');
  process.exit(0);
}

const env = msvcEnv('x86');
if (!env) {
  console.log('[build-fllhost] no 32-bit Visual C++ here: skipping the library host');
  process.exit(0);
}

mkdirSync(outDir, { recursive: true });
mkdirSync(sampleDir, { recursive: true });
mkdirSync(work, { recursive: true });

function cl(args, label) {
  const r = spawnSync('cl.exe', args, { cwd: work, env, stdio: 'inherit', shell: false });
  if (r.status !== 0) {
    console.error(`[build-fllhost] ${label} failed`);
    process.exit(r.status ?? 1);
  }
}

cl(
  [
    '/nologo',
    '/W3',
    '/O2',
    // the C runtime goes inside the executable. This binary is shipped, and an installed copy
    // of FoxDev Studio must not need a Visual C++ redistributable to load a library
    '/MT',
    '/EHa', // the host catches a library's access violation instead of dying with it
    '/D',
    'WIN32',
    '/D',
    'NDEBUG',
    '/D',
    '_CRT_SECURE_NO_WARNINGS',
    join(root, 'native', 'fllhost', 'fllhost.c'),
    `/Fe${join(outDir, 'fllhost.exe')}`,
    '/link',
    '/SUBSYSTEM:CONSOLE',
    '/MACHINE:X86',
  ],
  'the library host',
);
console.log(`[build-fllhost] ${join(outDir, 'fllhost.exe')}`);

// The Visual C++ 7.1 runtimes, beside the host.
//
// Windows searches the directory of the process doing the loading, and for a .fll that is the
// host and not Electron - so a library built with Visual Studio .NET 2003, which every .fll of
// that era was, finds its runtime here with nothing installed on the machine. SysWOW64 is where
// 64-bit Windows keeps the 32-bit copies, which is the pair a 32-bit library needs.
//
// The pair is tracked in git as well, because a CI runner has no Visual FoxPro to take them
// from: a copy already in the folder is left alone, and SysWOW64 is only read for one that
// is not there.
const RUNTIMES = ['msvcr71.dll', 'msvcp71.dll'];
const system32 = join(process.env['SystemRoot'] ?? 'C:/Windows', 'SysWOW64');
const carried = [];
const present = [];
for (const name of RUNTIMES) {
  const to = join(outDir, name);
  if (existsSync(to)) {
    present.push(name);
    continue;
  }
  const from = join(system32, name);
  if (!existsSync(from)) continue;
  copyFileSync(from, to);
  carried.push(`${name} (${statSync(from).size} bytes)`);
}
if (carried.length + present.length === RUNTIMES.length) {
  const said = [];
  if (present.length) said.push(`${present.join(', ')} already here`);
  if (carried.length) said.push(`carrying ${carried.join(', ')} from ${system32}`);
  console.log(`[build-fllhost] ${said.join('; ')}`);
} else {
  console.log(
    `[build-fllhost] the Visual C++ 7.1 runtimes are neither here nor in ${system32}: a library built against them will say so when it is loaded`,
  );
}

// The two sample libraries. hello.c calls _PutStr and takes nothing; reverse.c takes a string
// parameter and answers with one, so between them they cover the parameter block and the return
// path. Their source is Microsoft's, shipped with the product, so they are built here rather
// than copied into the repository.
const samples = join(
  process.env['ProgramFiles(x86)'] ?? 'C:/Program Files (x86)',
  'Microsoft Visual FoxPro 9',
  'Samples',
  'API',
);
if (!existsSync(join(samples, 'pro_ext.h'))) {
  console.log('[build-fllhost] no Visual FoxPro API samples here: no sample libraries built');
} else {
  // and one of our own, for what neither sample does: a variable passed by reference
  for (const [source, name] of [
    [join(samples, 'hello.c'), 'hello.fll'],
    [join(samples, 'reverse.c'), 'reverse.fll'],
    [join(root, 'tests', 'fll', 'refparm.c'), 'refparm.fll'],
  ]) {
    cl(
      [
        '/nologo',
        '/W3',
        '/O2',
        '/MD',
        '/Gr', // a library's functions are __fastcall: the host calls them with ECX
        '/LD',
        // pro_ext.h packs to a byte and then includes windows.h inside that packing, which
        // today's winnt.h refuses with a static assertion on its own structure sizes. Pulling
        // windows.h in first leaves pro_ext.h's own include of it a no-op.
        '/FIwindows.h',
        '/D',
        'WIN32',
        '/D',
        'NDEBUG',
        `/I${samples}`,
        source,
        `/Fe${join(sampleDir, name)}`,
        '/link',
        `/LIBPATH:${samples}`,
        'winapims.lib',
        'kernel32.lib',
        'user32.lib',
        '/MACHINE:X86',
      ],
      name,
    );
    console.log(`[build-fllhost] ${join(sampleDir, name)}`);
  }
}

rmSync(work, { recursive: true, force: true });
