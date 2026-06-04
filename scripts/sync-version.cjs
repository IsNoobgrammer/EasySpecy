// sync-version.cjs - Reads VERSION.txt and syncs version to all config files
const fs = require('fs');
const path = require('path');

const rootDir = path.resolve(__dirname, '..');
const versionFile = path.join(rootDir, 'VERSION.txt');

// Read version from VERSION.txt
const version = fs.readFileSync(versionFile, 'utf8').trim();
console.log(`📦 Syncing version: ${version}`);

// Update package.json
const packageJsonPath = path.join(rootDir, 'package.json');
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
packageJson.version = version;
fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
console.log(`✅ Updated package.json → ${version}`);

// Update src-tauri/Cargo.toml
const cargoTomlPath = path.join(rootDir, 'src-tauri', 'Cargo.toml');
let cargoToml = fs.readFileSync(cargoTomlPath, 'utf8');
cargoToml = cargoToml.replace(/^version = \{.*\}$/m, `version = "${version}"`);
fs.writeFileSync(cargoTomlPath, cargoToml);
console.log(`✅ Updated Cargo.toml → ${version}`);

console.log('✨ Version sync complete!');
