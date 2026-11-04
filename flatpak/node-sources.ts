// Writes flatpak/node-sources.json from app/bun.lock: one archive source per npm package, extracted
// into the Bun cache that the manifest's offline `bun install` reads
// (flatpak-node/bun-cache/<name>@<version>@@@1, the layout Bun's own cache uses).
// flatpak-builder-tools has no generator for bun.lock, hence this one.
//
// Only what Bun installs inside the Flatpak's SDK: Linux, glibc, x86_64 or aarch64 (a package for
// one of them gets only-arches). Bun reads the lock's os/cpu/libc fields the same way.
//
//   bun flatpak/node-sources.ts      (flatpak/update-sources.sh runs it)

type Meta = { os?: string | string[]; cpu?: string | string[]; libc?: string | string[] };
type Entry = [id: string, registry: string, meta: Meta, integrity: string];

const root = new URL("..", import.meta.url).pathname;
const lock = Bun.JSONC.parse(await Bun.file(`${root}app/bun.lock`).text()) as {
  packages: Record<string, Entry>;
};

// npm's os/cpu/libc: a list of allowed values or of excluded ones ("!win32"); Bun writes "none"
// when nothing matches.
const allows = (field: string | string[] | undefined, value: string) => {
  if (field === undefined) return true;
  const values = Array.isArray(field) ? field : [field];
  if (values.includes(value)) return true;
  if (values.includes(`!${value}`)) return false;
  return values.length > 0 && values.every((v) => v.startsWith("!"));
};

const arches = { x64: "x86_64", arm64: "aarch64" } as const;

const sources = new Map<string, Record<string, unknown>>();
for (const [id, registry, meta, integrity] of Object.values(lock.packages)) {
  if (registry !== "") throw new Error(`${id}: only packages from the npm registry are supported`);
  if (!integrity?.startsWith("sha512-")) throw new Error(`${id}: no sha512 integrity`);
  if (!allows(meta.os, "linux") || !allows(meta.libc, "glibc")) continue;
  const onlyArches = Object.entries(arches)
    .filter(([cpu]) => allows(meta.cpu, cpu))
    .map(([, arch]) => arch);
  if (onlyArches.length === 0) continue;
  const at = id.lastIndexOf("@");
  const name = id.slice(0, at);
  const version = id.slice(at + 1);
  const file = `${name.split("/").pop()}-${version}.tgz`;
  sources.set(id, {
    type: "archive",
    url: `https://registry.npmjs.org/${name}/-/${file}`,
    sha512: Buffer.from(integrity.slice("sha512-".length), "base64").toString("hex"),
    dest: `flatpak-node/bun-cache/${id}@@@1`,
    ...(onlyArches.length < Object.keys(arches).length ? { "only-arches": onlyArches } : {}),
  });
}

const sorted = [...sources.entries()].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)).map(([, s]) => s);
await Bun.write(`${root}flatpak/node-sources.json`, `${JSON.stringify(sorted, null, 4)}\n`);
console.log(`flatpak/node-sources.json: ${sorted.length} packages`);
