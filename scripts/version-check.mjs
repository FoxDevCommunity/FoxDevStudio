// Says which version a build is, and refuses a release whose tag disagrees with package.json.
//
// A release is a deliberate thing: bump package.json, commit, tag `v<version>`, push the tag.
// When this runs on a tag (GitHub sets GITHUB_REF_TYPE=tag and GITHUB_REF_NAME to its name)
// the two must agree, or nothing is built and nothing is uploaded. Anywhere else it only
// prints the version, so the workflow log shows what was stamped.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const { version } = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'));

if (process.env.GITHUB_REF_TYPE === 'tag') {
  const tag = process.env.GITHUB_REF_NAME ?? '';
  const tagged = tag.replace(/^v/, '');
  if (tagged !== version) {
    console.error(`[version-check] tag ${tag} says ${tagged} but package.json says ${version}: bump package.json, commit, and tag again`);
    process.exit(1);
  }
  console.log(`[version-check] release ${version}, from tag ${tag}`);
} else {
  console.log(`[version-check] ${version}`);
}
