//! Range-backed standalone audio. FilmCraft's bootstrap codecs are reused without loading a whole song.
use filmcraft_frame::{AudioBuffer, VideoFrame};
use filmcraft_media::{
    AudioStreamInfo, FrameRequest, MediaError, MediaInfo, MediaKind, MediaSource,
    reader::SharedReader,
};
use filmcraft_time::Tick;
use std::{
    io::{self, Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{Decoder, DecoderOptions},
    formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::{MediaSource as SymSource, MediaSourceStream},
    meta::MetadataOptions,
    probe::Hint,
};
struct RangeFile {
    reader: SharedReader,
    position: u64,
}
impl Read for RangeFile {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = out
            .len()
            .min(self.reader.len().saturating_sub(self.position) as usize);
        self.reader.read_at(self.position, &mut out[..n])?;
        self.position += n as u64;
        Ok(n)
    }
}
impl Seek for RangeFile {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let p = match from {
            SeekFrom::Start(p) => i128::from(p),
            SeekFrom::Current(p) => i128::from(self.position) + i128::from(p),
            SeekFrom::End(p) => i128::from(self.reader.len()) + i128::from(p),
        };
        if p < 0 || p > i128::from(self.reader.len()) {
            return Err(io::Error::other("Ogiltigt ljudintervall"));
        }
        self.position = p as u64;
        Ok(self.position)
    }
}
impl SymSource for RangeFile {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.reader.len())
    }
}
struct DecoderState {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: u32,
}
pub struct StreamAudio {
    info: MediaInfo,
    rate: u32,
    channels: usize,
    state: Mutex<DecoderState>,
}
impl StreamAudio {
    pub fn open(name: &str, reader: SharedReader) -> Result<Self, String> {
        let bytes = reader.len();
        let ext = name.rsplit('.').next().unwrap_or("");
        let mut hint = Hint::new();
        hint.with_extension(ext);
        let stream = MediaSourceStream::new(
            Box::new(RangeFile {
                reader,
                position: 0,
            }),
            Default::default(),
        );
        let mut format = symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions {
                    enable_gapless: true,
                    ..Default::default()
                },
                &MetadataOptions::default(),
            )
            .map_err(|e| e.to_string())?
            .format;
        let track = format.default_track().ok_or("Ljudspåret saknas")?.clone();
        let rate = track
            .codec_params
            .sample_rate
            .ok_or("Ljudfrekvensen saknas")?;
        let channels = track
            .codec_params
            .channels
            .ok_or("Ljudkanaler saknas")?
            .count();
        if !(8000..=192000).contains(&rate) || channels == 0 || channels > 6 {
            return Err("Ljudformatets frekvens eller kanaler stöds inte".into());
        }
        let frames = if let Some(n) = track.codec_params.n_frames {
            n
        } else {
            let mut frames = 0;
            let mut packets = 0;
            while let Ok(p) = format.next_packet() {
                if p.track_id() == track.id {
                    frames = frames.max(p.ts().saturating_add(p.dur()));
                }
                packets += 1;
                if packets > 2_000_000 {
                    return Err("Ljudfilens index är för stort".into());
                }
            }
            format
                .seek(
                    SeekMode::Accurate,
                    SeekTo::TimeStamp {
                        ts: 0,
                        track_id: track.id,
                    },
                )
                .map_err(|e| e.to_string())?;
            frames
        };
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| e.to_string())?;
        let codec = symphonia::default::get_codecs()
            .get_codec(track.codec_params.codec)
            .map(|c| c.short_name.to_uppercase())
            .unwrap_or_else(|| ext.to_uppercase());
        let info = MediaInfo {
            name: name.into(),
            kind: MediaKind::AudioOnly,
            duration: Tick::from_units(frames.min(i64::MAX as u64) as i64, i64::from(rate)),
            video: None,
            audio: Some(AudioStreamInfo {
                sample_rate: rate,
                channels: channels as u32,
                codec,
                bits_per_sample: track.codec_params.bits_per_sample,
            }),
            container: ext.to_uppercase(),
            start_timecode: None,
            file_size: Some(bytes),
        };
        Ok(Self {
            info,
            rate,
            channels,
            state: Mutex::new(DecoderState {
                format,
                decoder,
                track: track.id,
            }),
        })
    }
}
impl MediaSource for StreamAudio {
    fn info(&self) -> &MediaInfo {
        &self.info
    }
    fn video_frame(&self, _: FrameRequest) -> Result<Arc<VideoFrame>, MediaError> {
        Err(MediaError::NoStream("video"))
    }
    fn audio(&self, start: i64, frames: usize, rate: u32) -> Result<AudioBuffer, MediaError> {
        if start < 0 || frames > 1_600_000 || !(8000..=192000).contains(&rate) {
            return Err(MediaError::Decode("Ogiltigt ljudintervall".into()));
        }
        let begin = (start as f64 * f64::from(self.rate) / f64::from(rate)).floor() as u64;
        let count = (frames as f64 * f64::from(self.rate) / f64::from(rate)).ceil() as usize + 3;
        if count
            .checked_mul(self.channels)
            .is_none_or(|n| n > 48_000_000)
        {
            return Err(MediaError::Decode("Ljudbufferten är för stor".into()));
        }
        let mut source = vec![vec![0.0; count]; self.channels];
        let mut state = self
            .state
            .lock()
            .map_err(|_| MediaError::Decode("Ljudavkodaren är upptagen".into()))?;
        let track = state.track;
        state
            .format
            .seek(
                SeekMode::Accurate,
                SeekTo::TimeStamp {
                    ts: begin.saturating_sub(u64::from(self.rate) / 4),
                    track_id: track,
                },
            )
            .map_err(|e| MediaError::Decode(e.to_string()))?;
        state.decoder.reset();
        let mut packets = 0;
        while let Ok(packet) = state.format.next_packet() {
            if packet.track_id() != track {
                continue;
            }
            if packet.ts() > begin + count as u64 {
                break;
            }
            packets += 1;
            if packets > 100_000 {
                return Err(MediaError::Decode(
                    "Ljudintervallet kan inte avkodas".into(),
                ));
            }
            let buffer = state
                .decoder
                .decode(&packet)
                .map_err(|e| MediaError::Decode(e.to_string()))?;
            if buffer.capacity() > 262_144 || buffer.spec().channels.count() != self.channels {
                return Err(MediaError::Decode("Ogiltig ljudpaketstorlek".into()));
            }
            let mut samples = SampleBuffer::<f32>::new(buffer.capacity() as u64, *buffer.spec());
            samples.copy_interleaved_ref(buffer);
            for (i, frame) in samples.samples().chunks_exact(self.channels).enumerate() {
                let position = i128::from(packet.ts()) + i as i128 - i128::from(begin);
                if position >= 0 && (position as usize) < count {
                    for (ch, &v) in frame.iter().enumerate() {
                        source[ch][position as usize] = v;
                    }
                }
            }
        }
        let mut channels = vec![vec![0.0; frames]; self.channels];
        for (channel, input) in channels.iter_mut().zip(&source) {
            for (i, value) in channel.iter_mut().enumerate() {
                let p = (start as f64 + i as f64) * f64::from(self.rate) / f64::from(rate)
                    - begin as f64;
                let j = p.floor() as usize;
                let f = (p - j as f64) as f32;
                *value = input.get(j).copied().unwrap_or(0.0) * (1.0 - f)
                    + input.get(j + 1).copied().unwrap_or(0.0) * f;
            }
        }
        Ok(AudioBuffer {
            sample_rate: rate,
            channels,
        })
    }
}
