# Generated Moonshine runtime notices

`../../moonshine-runtime.json` is the checked-in provenance manifest for the native
Moonshine runtime. `scripts/prepare_moonshine_macos.sh` populates this directory
with the exact license/notice texts copied from the pinned Moonshine source tree
before a macOS application bundle is built.

Generated notice files are intentionally not committed. The macOS bundle job
fails if preparation did not create the provenance inventory and license set.
