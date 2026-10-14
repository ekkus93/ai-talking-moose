use super::system_tts_pcm::SystemPcmAccumulator;

#[test]
fn native_pcm_formats_produce_identical_mono_playback_and_preserve_rate() {
    let mut outputs = Vec::new();
    let mut float_interleaved = SystemPcmAccumulator::new(44_100).unwrap();
    float_interleaved
        .append_interleaved_f32(&[0.0, 0.0, 0.5, 0.5], 2)
        .unwrap();
    outputs.push(float_interleaved.finish().unwrap());

    let mut float_planar = SystemPcmAccumulator::new(44_100).unwrap();
    float_planar
        .append_planar_f32(&[&[0.0, 0.5], &[0.0, 0.5]])
        .unwrap();
    outputs.push(float_planar.finish().unwrap());

    let mut double_planar = SystemPcmAccumulator::new(44_100).unwrap();
    double_planar
        .append_planar_f64(&[&[0.0, 0.5], &[0.0, 0.5]])
        .unwrap();
    outputs.push(double_planar.finish().unwrap());

    let mut integer_interleaved = SystemPcmAccumulator::new(44_100).unwrap();
    integer_interleaved
        .append_interleaved_i16(&[0, 0, 16_384, 16_384], 2)
        .unwrap();
    outputs.push(integer_interleaved.finish().unwrap());

    let mut integer_planar = SystemPcmAccumulator::new(44_100).unwrap();
    integer_planar
        .append_planar_i32(&[
            &[0, 1_073_741_824],
            &[0, 1_073_741_824],
        ])
        .unwrap();
    outputs.push(integer_planar.finish().unwrap());

    let expected = [0_i16, 16_384_i16].map(i16::to_le_bytes).concat();
    for output in outputs {
        assert_eq!(output.sample_rate, 44_100);
        assert_eq!(output.pcm_bytes, expected);
    }
}
