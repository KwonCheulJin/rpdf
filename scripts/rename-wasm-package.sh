#!/usr/bin/env bash
set -euo pipefail
node -e "
  const fs = require('fs');
  const p = JSON.parse(fs.readFileSync('npm/core/package.json', 'utf8'));
  p.name = '@rpdf/core';
  fs.writeFileSync('npm/core/package.json', JSON.stringify(p, null, 2) + '\n');
"
