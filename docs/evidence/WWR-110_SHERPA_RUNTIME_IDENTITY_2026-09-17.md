# WWR-110 — sherpa native runtime identity and packaging evidence

Wake Word V1 pins sherpa-onnx native runtime **v1.13.8** and supports only the two architectures named below until real acceptance expands that set.

The deterministic identity freezer ran successfully on exact master `4955d1f1872c310f872219b812f5d95a3887e97e` in GitHub Actions run `35294590820`. The freezer downloaded the upstream v1.13.8 native JARs, rejected unsafe archive members, identified native binary architecture from file headers, and hashed the archive and every native library that Wake Word packaging is permitted to consume.

## Frozen identities

### Linux x86_64

- Archive: `sherpa-onnx-native-lib-linux-x64-1.13.8.jar`
- Archive bytes: `10515871`
- Archive SHA-256: `30c93b59381113f9c20aedbbf9fc1ad399158f6bc03dddc0f8934a6e28e069ba`
- `libonnxruntime.so`: `27026609` bytes, SHA-256 `4b3607aebd1784b26b6f9b20e4bd974c7ab8287043e4d095cb7d2cb40b5e566e`
- `libsherpa-onnx-jni.so`: `5166360` bytes, SHA-256 `adcabd1866f667ec78796a504ff96030eff30fbd80792e892752c64a861bf231`
- Required architecture: ELF x86_64
- Deterministic installed root: `runtime/sherpa-onnx/v1.13.8/linux-x86_64`
- Deterministic bundle root: `resources/runtime/sherpa-onnx/v1.13.8/linux-x86_64`

### macOS arm64

- Archive: `sherpa-onnx-native-lib-osx-aarch64-1.13.8.jar`
- Archive bytes: `9460832`
- Archive SHA-256: `42e272180c8836127f024f3335d7afcdfb30fe0164b78330d5d449034e34ce34`
- `libonnxruntime.dylib`: `29006384` bytes, SHA-256 `b0613d0ae53199a83b05fa48e169211498e9d40d54beaa372068ebe5ec5b0929`
- `libsherpa-onnx-jni.dylib`: `4218024` bytes, SHA-256 `e8025656a2680b838dd7ccd7d7ee7e88e5da42a35ad010d8717c22dd7b851ca1`
- Required architecture: Mach-O arm64
- Deterministic installed root: `runtime/sherpa-onnx/v1.13.8/macos-arm64`
- Deterministic bundle root: `resources/runtime/sherpa-onnx/v1.13.8/macos-arm64`

## Enforcement

`wake-word-artifacts.json` is authoritative for runtime version, platform allow-list, archive identity, consumed-library identities, architecture, installed path, bundle path, and license notice.

`scripts/prepare_wake_word_runtime.py` verifies cached archives before every extraction, rejects unsafe archive paths, verifies each consumed library's size/SHA-256 and native architecture before installation, and re-verifies an already-installed runtime instead of trusting cache presence. It supports an explicitly supplied archive for deterministic offline preparation. Unsupported platforms and failures are reported without absolute paths or archive contents.

`tests/test_wake_word_runtime_artifacts.py` exercises wrong architecture, corrupt archive, corrupt consumed library, missing library, path traversal, cache tampering/re-verification, deterministic offline preparation, and sanitized unsupported-platform behavior.

The runtime is Apache-2.0 licensed; attribution is recorded in `docs/licenses/SHERPA_ONNX_RUNTIME_NOTICE.md`. Model licensing remains separately recorded in WWR-100 evidence.
