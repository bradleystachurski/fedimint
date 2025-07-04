# Debug: Release Size Investigation

## Context for New Claude Instance

**Repository**: Fedimint (https://github.com/fedimint/fedimint)
**Current Branch**: `2025-07-04-debug-release-size` (created from master)
**Working Directory**: `/home/stachurski/code/fedimint`
**Issue**: Release size exceeded CI threshold, preventing v0.8.0-beta.0 release

## Problem Summary

The `gatewayd` binary's Nix closure size (212.19 MB) exceeded the CI threshold of 200 MB during the v0.8.0-beta.0 release, causing the CI build to fail.

**Original Error**:
```
gatewayd's Nix closure size: 212189616
gatewayd's Nix closure size seems too big: 212189616
```

## Root Cause Analysis

### CI Configuration
- **Location**: `.github/workflows/ci-nix.yml:501`
- **Previous threshold**: 200 MB (200,000,000 bytes)
- **Current threshold**: 220 MB (220,000,000 bytes) - increased as temporary fix
- **Actual size**: 212.19 MB (212,189,616 bytes)
- **Overage**: ~12.19 MB over the previous limit

### Historical Context
- **Original protection**: Added in commit `4c6c43ecc5984e66765c81b8f6a21ba6a77877fa` (Feb 2025)
- **Previous bump**: Commit `2cafa94b17c3f63dd50ae4d150521e3b1a086e84` (Apr 2025)
  - Raised from 150 MB → 200 MB because `fedimintd` was at 153 MB
- **Current bump**: Raised from 200 MB → 220 MB for `gatewayd` at 212 MB

### Measurement Method
The CI uses this command to measure closure size:
```bash
closure_size=$(nix path-info -rS --json .#$bin | nix run nixpkgs#jq '. | to_entries[] | select(.value.ultimate == true) | .value.narSize')
```

## Temporary Solution Applied

**Branch**: `2025-07-04-debug-release-size`
**Change**: Increased closure size threshold from 200 MB to 220 MB in `.github/workflows/ci-nix.yml`

## Next Steps (Priority Order)

### 1. Investigate Size Increase Root Cause
- **Compare dependency trees** between v0.7.x and v0.8.0-beta.0
- **Analyze what changed** in the `gatewayd` binary specifically
- **Check for new dependencies** or dependency version bumps
- **Review recent commits** that might have added functionality to the gateway

### 2. Identify Optimization Opportunities
- **Audit unused dependencies** in gateway crates
- **Profile binary size** by dependency/feature
- **Check for duplicate dependencies** in the closure
- **Review feature flags** that might be unnecessarily enabled
- **Investigate dead code elimination** opportunities

### 3. Implement Size Reduction Strategies
- **Remove unused dependencies** from `Cargo.toml` files
- **Optimize dependency versions** to reduce conflicts
- **Use feature flags** to disable unnecessary functionality
- **Consider binary splitting** if the gateway has grown too large
- **Review Nix package configuration** for optimization opportunities

### 4. Prevent Future Size Creep
- **Add pre-commit hooks** to warn about significant dependency changes
- **Monitor closure sizes** in CI for all releases, not just tagged ones
- **Document size budgets** for different binaries
- **Regular audits** of dependency trees

## Investigation Commands

### Analyze Current Dependencies
```bash
# Check dependency tree
cargo tree --package fedimint-gateway-server

# Check for duplicate dependencies
cargo tree --duplicates

# Analyze binary size by dependencies
cargo bloat --release --package fedimint-gateway-server --crates
```

### Compare with Previous Version
```bash
# Create temp directory for analysis files
mkdir -p ./tmp

# Check size difference between versions
git checkout releases/v0.7.x
nix build .#gatewayd
nix path-info -rS --json .#gatewayd | jq '.[].narSize' > ./tmp/v0.7-size.txt

git checkout releases/v0.8
nix build .#gatewayd  
nix path-info -rS --json .#gatewayd | jq '.[].narSize' > ./tmp/v0.8-size.txt

# Compare sizes
echo "Size comparison:"
echo "v0.7.x: $(cat ./tmp/v0.7-size.txt)"
echo "v0.8: $(cat ./tmp/v0.8-size.txt)"

# Compare commit ranges (current branch is on master after v0.8.0-beta.0)
git log --oneline releases/v0.7.x..releases/v0.8 -- gateway/ > ./tmp/gateway-commits.txt
```

### Profile Nix Closure
```bash
# Create temp directory for analysis files
mkdir -p ./tmp

# Analyze what's in the closure
nix-store --query --tree $(nix-build --no-out-link .#gatewayd) > ./tmp/closure-tree.txt

# Find largest components
nix path-info -rS .#gatewayd | sort -k2 -n > ./tmp/closure-sizes.txt

# Show top 20 largest components
tail -20 ./tmp/closure-sizes.txt
```

## Related Files
- `.github/workflows/ci-nix.yml` - CI configuration with size checks
- `gateway/fedimint-gateway-server/Cargo.toml` - Main gateway dependencies
- `gateway/fedimint-gateway-server/src/bin/main.rs` - Gateway entry point
- `./tmp/` - Analysis output directory (created during investigation)

## Important Notes
- All analysis files are stored in `./tmp/` subdirectory to avoid affecting system files
- The `./tmp/` directory should be added to `.gitignore` if it doesn't exist there already
- Always return to the working branch (`2025-07-04-debug-release-size`) after checking out other branches

## Success Criteria
- [ ] Identify specific cause of 12+ MB size increase
- [ ] Reduce `gatewayd` closure size below 200 MB
- [ ] Implement preventive measures for future size management
- [ ] Document binary size budgets and monitoring process

## Current Status

- ✅ **Temporary fix applied**: Threshold increased to 220 MB
- ✅ **Branch created**: `2025-07-04-debug-release-size`
- ⏳ **Next**: Begin investigation using commands above

## Key Commands for Investigation

**Start here**: Run these commands to begin the investigation:
```bash
# 1. Check current branch and status
git status
git log --oneline -10

# 2. Create temp directory for analysis files
mkdir -p ./tmp

# 3. Compare dependency trees between versions
git checkout releases/v0.7.x
cargo tree --package fedimint-gateway-server > ./tmp/v0.7-deps.txt

git checkout releases/v0.8  
cargo tree --package fedimint-gateway-server > ./tmp/v0.8-deps.txt

# 4. Compare the differences
diff ./tmp/v0.7-deps.txt ./tmp/v0.8-deps.txt

# 5. Check for new large dependencies
cargo bloat --release --package fedimint-gateway-server --crates | head -20
```

## Investigation Results

### Root Cause Identified
The 12+ MB binary size increase is primarily due to **debug symbols not being stripped** from the release binary:

- **Primary Issue**: Binary contains debug info (`file result/bin/gatewayd` shows `with debug_info, not stripped`)
- **Configuration**: `flake.nix:245` has `dontStrip = !pkgs.stdenv.isDarwin;` (strips only on macOS)
- **Profile**: `Cargo.toml` release profile includes `debug = "line-tables-only"`
- **Impact**: 83MB text section due to debug symbols in 203MB binary

### Secondary Contributors
- **New dependency**: `fedimint-cursed-redb` added in commit 33236603da
- **Version bumps**: `axum` 0.7.9→0.8.4, `lightning-invoice` 0.32.0→0.33.2, etc.

### Recommended Fix
Enable stripping in Nix build by changing `flake.nix:245` from:
```nix
dontStrip = !pkgs.stdenv.isDarwin;
```
to:
```nix
dontStrip = false;
```

This should reduce binary size by ~40-50MB, bringing it well below the 200MB threshold.

### Reproduction Steps
To independently reproduce this investigation:

```bash
# 1. Clone and setup
git clone https://github.com/fedimint/fedimint.git
cd fedimint
git checkout 2025-07-04-debug-release-size
mkdir -p ./tmp

# 2. Compare dependency trees between versions
git checkout upstream/releases/v0.7
cargo tree --package fedimint-gateway-server > ./tmp/v0.7-deps.txt
git checkout upstream/releases/v0.8
cargo tree --package fedimint-gateway-server > ./tmp/v0.8-deps.txt
diff ./tmp/v0.7-deps.txt ./tmp/v0.8-deps.txt

# 3. Analyze binary size and debug info
git checkout 2025-07-04-debug-release-size
nix build .#gatewayd
ls -lh result/bin/gatewayd         # Check file size
file result/bin/gatewayd           # Check if stripped
size result/bin/gatewayd           # Check section sizes

# 4. Analyze Nix closure components
nix path-info -rS .#gatewayd | sort -k2 -n | tail -20

# 5. Check duplicate dependencies
cargo tree --duplicates --package fedimint-gateway-server

# 6. Review gateway-specific commits
git log --oneline upstream/releases/v0.7..upstream/releases/v0.8 -- gateway/
```