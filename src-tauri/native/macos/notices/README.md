# Generated macOS release notices

`../../moonshine-runtime.json` is the checked-in provenance manifest for the native
Moonshine runtime. `scripts/prepare_moonshine_macos.sh` populates this directory
with the exact Moonshine/native license and notice texts copied from the pinned
source tree and also stages the Talking Moose project license.

For a tagged distribution, `scripts/collect_release_licenses.py` additionally
collects the resolved production npm and Rust dependency license/notice files into
`Dependencies/` and writes `Dependencies/DEPENDENCY_LICENSES.md`.

Generated notice files are intentionally not committed. Release verification fails
if preparation did not create the project license, native provenance inventory,
Moonshine notice set, and dependency notice inventory.
