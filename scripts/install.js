#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');

const REPO = 'aezizhu/a2zaiusage';
const BINARY_NAME = process.platform === 'win32' ? 'a2zusage.exe' : 'a2zusage';

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === 'darwin') {
    return arch === 'arm64' ? 'darwin-arm64' : 'darwin-x64';
  } else if (platform === 'linux') {
    return arch === 'arm64' ? 'linux-arm64' : 'linux-x64';
  } else if (platform === 'win32') {
    return 'windows-x64';
  }

  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

async function downloadFile(url, destPath) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destPath);

    const request = (url) => {
      https.get(url, (response) => {
        if (response.statusCode === 302 || response.statusCode === 301) {
          request(response.headers.location);
          return;
        }

        if (response.statusCode !== 200) {
          reject(new Error(`Failed to download: ${response.statusCode}`));
          return;
        }

        response.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve();
        });
      }).on('error', reject);
    };

    request(url);
  });
}

async function getLatestRelease() {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: 'api.github.com',
      path: `/repos/${REPO}/releases/latest`,
      headers: { 'User-Agent': 'a2zusage-installer' }
    };

    https.get(options, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          resolve(JSON.parse(data));
        } catch (e) {
          reject(e);
        }
      });
    }).on('error', reject);
  });
}

async function install() {
  const binDir = path.join(__dirname, '..', 'bin');
  const binaryPath = path.join(binDir, BINARY_NAME);

  // Check if binary already exists
  if (fs.existsSync(binaryPath)) {
    console.log('a2zusage binary already installed');
    return;
  }

  const platformKey = getPlatformKey();
  console.log(`Installing a2zusage for ${platformKey}...`);

  try {
    const release = await getLatestRelease();
    const assetName = `a2zusage-${platformKey}${process.platform === 'win32' ? '.exe' : ''}`;
    const asset = release.assets?.find(a => a.name === assetName);

    if (asset) {
      await downloadFile(asset.browser_download_url, binaryPath);
      if (process.platform !== 'win32') {
        fs.chmodSync(binaryPath, '755');
      }
      console.log('a2zusage installed successfully!');
    } else {
      console.log('Pre-built binary not found. Building from source...');
      buildFromSource(binDir);
    }
  } catch (error) {
    console.log('Failed to download binary, building from source...');
    buildFromSource(binDir);
  }
}

function buildFromSource(binDir) {
  try {
    // Check if Rust is installed
    execSync('cargo --version', { stdio: 'ignore' });

    const projectRoot = path.join(__dirname, '..');
    execSync('cargo build --release', {
      cwd: projectRoot,
      stdio: 'inherit'
    });

    const sourceBinary = path.join(projectRoot, 'target', 'release', BINARY_NAME);
    const destBinary = path.join(binDir, BINARY_NAME);

    fs.copyFileSync(sourceBinary, destBinary);
    if (process.platform !== 'win32') {
      fs.chmodSync(destBinary, '755');
    }

    console.log('a2zusage built and installed successfully!');
  } catch (error) {
    console.error('Error: Rust is required to build a2zusage from source.');
    console.error('Install Rust from https://rustup.rs/ and try again.');
    console.error('Or download a pre-built binary from:');
    console.error(`https://github.com/${REPO}/releases`);
    process.exit(1);
  }
}

install().catch(err => {
  console.error('Installation failed:', err.message);
  process.exit(1);
});
