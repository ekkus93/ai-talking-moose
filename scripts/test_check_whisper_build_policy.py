import unittest

from scripts.check_whisper_build_policy import BuildPolicyError, validate_build_policy_text


VALID_POLICY = """
const WHISPER_BUILD_JOBS: &str = "TALKING_MOOSE_WHISPER_BUILD_JOBS";

fn cpu_count() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
}

fn explicit_whisper_build_jobs() -> Option<usize> {
    None
}

fn whisper_build_jobs() -> usize {
    explicit_whisper_build_jobs().unwrap_or_else(cpu_count)
}

fn emit_whisper_native_rerun_paths(whisper_src: &Path) {
    for path in [
        whisper_src.join("CMakeLists.txt"),
        whisper_src.join("cmake"),
        whisper_src.join("include"),
        whisper_src.join("src"),
        whisper_src.join("ggml").join("CMakeLists.txt"),
        whisper_src.join("ggml").join("cmake"),
        whisper_src.join("ggml").join("include"),
        whisper_src.join("ggml").join("src"),
    ] {
        emit_rerun_tree(&path);
    }
}

fn build_whisper_from_source() {
    let threads = whisper_build_jobs().to_string();
}

fn main() {
    println!("cargo:rerun-if-env-changed={WHISPER_BUILD_JOBS}");
}
"""


class WhisperBuildPolicyTest(unittest.TestCase):
    def test_valid_policy_passes(self):
        validate_build_policy_text(VALID_POLICY)

    def test_proc_nproc_probe_fails(self):
        with self.assertRaisesRegex(BuildPolicyError, "/proc/nproc"):
            validate_build_policy_text(VALID_POLICY + '\nlet legacy = "/proc/nproc";\n')

    def test_missing_operator_override_fails(self):
        with self.assertRaisesRegex(BuildPolicyError, "parallelism override"):
            validate_build_policy_text(
                VALID_POLICY.replace(
                    'const WHISPER_BUILD_JOBS: &str = "TALKING_MOOSE_WHISPER_BUILD_JOBS";\n',
                    "",
                )
            )

    def test_missing_native_rerun_root_fails(self):
        with self.assertRaisesRegex(BuildPolicyError, "ggml.*include"):
            validate_build_policy_text(
                VALID_POLICY.replace(
                    '        whisper_src.join("ggml").join("include"),\n',
                    "",
                )
            )


if __name__ == "__main__":
    unittest.main()
