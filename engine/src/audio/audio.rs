use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle};
use synthrs::{music, synthesizer::make_samples};

pub struct SoundChannels {
    pub stream: OutputStream,
    pub stream_handle: OutputStreamHandle,
}

impl SoundChannels {
    pub fn new() -> SoundChannels {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        SoundChannels {
            stream,
            stream_handle,
        }
    }
}

pub struct Note {
    semitone: u8,
    octave: u8,
    beats: u8,
}

impl Note {
    pub fn new(semitone: u8, octave: u8, beats: u8) -> Self {
        Note {
            semitone,
            octave,
            beats,
        }
    }
    pub fn pitch(&self) -> f64 {
        music::note(440.0, self.semitone as usize, self.octave as usize)
    }
}

pub fn generate_samples<F, G>(notes: Vec<Note>, bpm: f32, waveform: F) -> SamplesBuffer<f32>
where
    F: Fn(f64) -> G,
    G: Fn(f64) -> f64,
{
    let multiplier = 60.0 / bpm;
    let num_samples = (notes
        .iter()
        .map(|x| x.beats as usize)
        .reduce(|x, y| x + y)
        .unwrap_or(0) as f32
        * multiplier)
        .floor() as usize;
    let mut samples: Vec<f64> = Vec::with_capacity(num_samples);
    for note in notes {
        samples.append(&mut make_samples(
            (note.beats as f32 * multiplier) as f64,
            44_100,
            waveform(note.pitch()),
        ));
    }
    SamplesBuffer::new(
        1,
        44_100,
        samples
            .iter()
            .map(|x| (*x as f32) / 10.0)
            .collect::<Vec<f32>>(),
    )
}
