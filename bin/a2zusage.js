#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const BINARY_NAME = process.platform === 'win32' ? 'a2zusage.exe' : 'a2zusage';
const binaryPath = path.join(__dirname, BINARY_NAME);

// Check if binary exists
if (!fs.existsSync(binaryPath)) {
  console.error('Error: a2zusage binary not found.');
  console.error('Try reinstalling: npm install -g a2zusage');
  process.exit(1);
}

// Pass all arguments to the binary
const args = process.argv.slice(2);
const child = spawn(binaryPath, args, {
  stdio: 'inherit',
  env: process.env
});

child.on('error', (err) => {
  console.error('Failed to run a2zusage:', err.message);
  process.exit(1);
});

child.on('exit', (code) => {
  process.exit(code || 0);
});
