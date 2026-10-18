use crate::error::{AppError, AppResult};
use crate::video::frame::Frame;
use ffmpeg_next as ffmpeg;
use ffmpeg::codec::decoder::Video as InnerDecoder;
use ffmpeg::format::context::Input;
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context as Scaler, Flags as ScaleFlags};
use ffmpeg::util::frame::video::Video as RawFrame;
use ffmpeg::util::format::Pixel;
use ffmpeg::Rational;
use std::path::{Path, PathBuf};
use std::time::Duration;

const METHOD_OPEN: &str = "video::decoder::VideoDecoder::open";
const METHOD_FRAME_AT: &str = "video::decoder::VideoDecoder::frame_at";
const METHOD_CONVERT: &str = "video::decoder::VideoDecoder::convert";

const EAGAIN_ERRNO: i32 = 11;

/// Opens a video file once and decodes frames on demand.
/// Keep the instance alive for the session — scrubbing is fast because the
/// file stays open and the decoder stays warm.
pub struct VideoDecoder {
    input: Input,
    decoder: InnerDecoder,
    scaler: Scaler,
    stream_index: usize,
    time_base: Rational,
    width: u32,
    height: u32,
    duration: Duration,
    path: PathBuf,
}

impl VideoDecoder {
    pub fn open(path: &Path) -> AppResult<Self> {
        ffmpeg::init()
            .map_err(|e| AppError::new(format!("{e}"), METHOD_OPEN, file!()))?;

        let input = ffmpeg::format::input(path)
            .map_err(|e| AppError::new(format!("{e}"), METHOD_OPEN, file!()))?;

        let stream = input
            .streams()
            .best(Type::Video)
            .ok_or_else(|| AppError::new("no video stream", METHOD_OPEN, file!()))?;

        let stream_index = stream.index();
        let time_base = stream.time_base();

        let codec_ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
            .map_err(|e| AppError::new(format!("{e}"), METHOD_OPEN, file!()))?;

        let decoder = codec_ctx
            .decoder()
            .video()
            .map_err(|e| AppError::new(format!("{e}"), METHOD_OPEN, file!()))?;

        let width = decoder.width();
        let height = decoder.height();

        let scaler = Scaler::get(
            decoder.format(),
            width,
            height,
            Pixel::RGBA,
            width,
            height,
            ScaleFlags::BILINEAR,
        )
        .map_err(|e| AppError::new(format!("{e}"), METHOD_OPEN, file!()))?;

        let duration = if input.duration() > 0 {
            Duration::from_micros(input.duration() as u64)
        } else {
            Duration::ZERO
        };

        Ok(Self {
            input,
            decoder,
            scaler,
            stream_index,
            time_base,
            width,
            height,
            duration,
            path: path.to_path_buf(),
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Seeks to the nearest keyframe at or before `timestamp` and decodes
    /// forward until a frame with pts >= `timestamp` is produced.
    /// Returns that frame as RGBA.
    pub fn frame_at(&mut self, timestamp: Duration) -> AppResult<Frame> {
        let target_us = timestamp.as_micros().min(i64::MAX as u128) as i64;

        self.input
            .seek(target_us, ..target_us)
            .map_err(|e| AppError::new(format!("{e}"), METHOD_FRAME_AT, file!()))?;

        self.decoder.flush();

        let stream_index = self.stream_index;
        let time_base = self.time_base;
        let mut found: Option<(RawFrame, Duration)> = None;

        {
            let decoder = &mut self.decoder;
            for (stream, packet) in self.input.packets() {
                if stream.index() != stream_index {
                    continue;
                }

                if let Err(e) = decoder.send_packet(&packet) {
                    let is_eagain =
                        matches!(&e, ffmpeg::Error::Other { errno } if *errno == EAGAIN_ERRNO);
                    if !is_eagain {
                        return Err(AppError::new(
                            format!("{e}"),
                            METHOD_FRAME_AT,
                            file!(),
                        ));
                    }
                }

                loop {
                    let mut decoded = RawFrame::empty();
                    if decoder.receive_frame(&mut decoded).is_err() {
                        break;
                    }

                    let pts = decoded.pts().unwrap_or(-1);
                    if pts < 0 {
                        continue;
                    }

                    let frame_ts = Duration::from_secs_f64(
                        pts as f64 * time_base.0 as f64 / time_base.1 as f64,
                    );

                    if frame_ts >= timestamp {
                        found = Some((decoded, frame_ts));
                        break;
                    }
                }

                if found.is_some() {
                    break;
                }
            }
        }

        let (frame, ts) = found.ok_or_else(|| {
            AppError::new(
                "no frame decoded at or after target timestamp",
                METHOD_FRAME_AT,
                file!(),
            )
        })?;

        self.convert(frame, ts)
    }

    fn convert(&mut self, raw: RawFrame, timestamp: Duration) -> AppResult<Frame> {
        let mut rgba_frame = RawFrame::empty();
        self.scaler
            .run(&raw, &mut rgba_frame)
            .map_err(|e| AppError::new(format!("{e}"), METHOD_CONVERT, file!()))?;

        let width_bytes = self.width as usize * 4;
        let stride = rgba_frame.stride(0);
        let src = rgba_frame.data(0);

        let mut rgba = Vec::with_capacity(width_bytes * self.height as usize);
        for row in 0..self.height as usize {
            let start = row * stride;
            let end = start + width_bytes;
            rgba.extend_from_slice(&src[start..end]);
        }

        Ok(Frame {
            width: self.width,
            height: self.height,
            rgba,
            timestamp,
        })
    }
}