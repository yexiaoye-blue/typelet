//! Desktop key clicks are mixed here, not in WebKit.
//!
//! WebKitGTK hands Web Audio to a PipeWire stream that can buffer for seconds.
//! This module owns one output stream, keeps the selected click as mono PCM, and
//! adds a voice for every key. Voices overlap and always play to the end. Word
//! pronunciation stays on the webview's media element.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig, SupportedBufferSize};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

const MAX_VOICES: usize = 32;
const PREFERRED_FRAMES: u32 = 256;
const TARGET_DB: f32 = -1.0;
const MAX_MAKEUP: f32 = 32.0;
// One rotated backup keeps the debug log from growing without bound.
const MAX_LOG_BYTES: u64 = 10 * 1024 * 1024;

pub struct KeyAudio {
    inner: Mutex<Option<Engine>>,
    // Set by the output stream's error callback so the next key rebuilds
    // the engine instead of staying silent forever.
    broken: Arc<AtomicBool>,
}

impl KeyAudio {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            broken: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadReport {
    frames: usize,
    rate: u32,
    device_rate: u32,
    channels: u16,
    makeup: f32,
    buffer_frames: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayReport {
    voice: u64,
}

struct Engine {
    commands: SyncSender<PlayCommand>,
    logs: &'static SyncSender<LogEvent>,
    clip: Mutex<Option<Arc<[f32]>>>,
    next_voice: AtomicU64,
    rate: u32,
    channels: u16,
    buffer_frames: u32,
    _stream: cpal::Stream,
}

struct Prepared {
    samples: Arc<[f32]>,
    source_rate: u32,
    frames: usize,
    makeup: f32,
}

struct PlayCommand {
    samples: Arc<[f32]>,
    gain: f32,
    id: u64,
    request: u64,
    enqueued: Instant,
    ipc_ms: u64,
}

struct Voice {
    samples: Arc<[f32]>,
    cursor: usize,
    gain: f32,
    id: u64,
    request: u64,
    enqueued: Instant,
    ipc_ms: u64,
    heard: bool,
}

struct AudioCallback {
    rx: Receiver<PlayCommand>,
    voices: Vec<Voice>,
    logs: SyncSender<LogEvent>,
    channels: usize,
    scratch: Vec<f32>,
    clock_logged: bool,
}

enum LogEvent {
    Clock {
        playback_us: Option<u64>,
        frames: usize,
    },
    Mixed {
        id: u64,
        request: u64,
        queue_us: u64,
        playback_us: Option<u64>,
        ipc_ms: u64,
        active: usize,
    },
    Finished {
        id: u64,
        request: u64,
        age_ms: u64,
    },
    Skip {
        id: u64,
        request: u64,
    },
    Queued {
        voice: u64,
        request: u64,
        gain: f32,
        ipc_ms: u64,
    },
    Stream(String),
    // A JSON line already rendered by the sender, for example from the webview.
    Line(String),
}

fn log_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join("key-sound.log"))
}

// One process-wide log thread owns the log file; every producer hands over a
// cheap enum so keystrokes never touch the filesystem.
static LOG_SENDER: OnceLock<SyncSender<LogEvent>> = OnceLock::new();

fn log_channel(app: &AppHandle) -> &'static SyncSender<LogEvent> {
    LOG_SENDER.get_or_init(|| {
        let Ok(path) = log_path(app) else {
            return detached_log_sender();
        };
        let (sender, receiver) = sync_channel::<LogEvent>(1024);
        let spawned = thread::Builder::new()
            .name("typelet-key-log".to_owned())
            .spawn(move || log_loop(path, receiver));
        match spawned {
            Ok(_handle) => sender,
            Err(_) => detached_log_sender(),
        }
    })
}

fn detached_log_sender() -> SyncSender<LogEvent> {
    // No usable log destination: hand out a sender whose receiver is gone so
    // every write fails fast and logging never blocks playback.
    let (sender, receiver) = sync_channel::<LogEvent>(1);
    drop(receiver);
    sender
}

pub fn log_frontend(app: &AppHandle, line: String) {
    let _ = log_channel(app).try_send(LogEvent::Line(line));
}

struct LogWriter {
    path: PathBuf,
    file: Option<File>,
    written: u64,
}

impl LogWriter {
    fn open(path: PathBuf) -> Self {
        let mut writer = Self {
            path,
            file: None,
            written: 0,
        };
        writer.reopen();
        writer
    }

    fn reopen(&mut self) {
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .ok();
        self.written = self
            .file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .map(|metadata| metadata.len())
            .unwrap_or(0);
    }

    fn rotate(&mut self) {
        self.file = None;
        let backup = self.path.with_extension("log.1");
        let _ = fs::rename(&self.path, &backup);
        self.reopen();
    }

    fn write(&mut self, line: String) {
        if self.file.is_none() {
            self.reopen();
        }
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let _ = writeln!(file, "{line}");
        let _ = file.flush();
        self.written += u64::try_from(line.len()).unwrap_or(0) + 1;
        if self.written > MAX_LOG_BYTES {
            self.rotate();
        }
    }
}

fn log_loop(path: PathBuf, receiver: Receiver<LogEvent>) {
    let mut writer = LogWriter::open(path);
    while let Ok(event) = receiver.recv() {
        writer.write(render_log_line(event));
    }
}

fn render_log_line(event: LogEvent) -> String {
    match event {
        LogEvent::Line(line) => line,
        LogEvent::Clock {
            playback_us,
            frames,
        } => event_line(&serde_json::json!({
            "event": "clock",
            "playbackUs": playback_us,
            "callbackFrames": frames,
        })),
        LogEvent::Mixed {
            id,
            request,
            queue_us,
            playback_us,
            ipc_ms,
            active,
        } => event_line(&serde_json::json!({
            "event": "mixed",
            "voice": id,
            "request": request,
            "queueUs": queue_us,
            "playbackUs": playback_us,
            "ipcMs": ipc_ms,
            "active": active,
        })),
        LogEvent::Finished {
            id,
            request,
            age_ms,
        } => event_line(&serde_json::json!({
            "event": "finished",
            "voice": id,
            "request": request,
            "ageMs": age_ms,
        })),
        LogEvent::Skip { id, request } => event_line(&serde_json::json!({
            "event": "skip-full",
            "voice": id,
            "request": request,
        })),
        LogEvent::Queued {
            voice,
            request,
            gain,
            ipc_ms,
        } => event_line(&serde_json::json!({
            "event": "queued",
            "voice": voice,
            "request": request,
            "gain": gain,
            "ipcMs": ipc_ms,
        })),
        LogEvent::Stream(message) => event_line(&serde_json::json!({
            "event": "stream-error",
            "message": message,
        })),
    }
}

fn event_line(extra: &serde_json::Value) -> String {
    let mut value = serde_json::json!({
        "t": iso_now(),
        "path": "native",
        "source": "mixer",
    });
    if let (Some(target), Some(extra)) = (value.as_object_mut(), extra.as_object()) {
        for (key, item) in extra {
            target.insert(key.clone(), item.clone());
        }
    }
    value.to_string()
}

#[tauri::command]
pub fn load_key_sound(
    app: AppHandle,
    state: State<'_, KeyAudio>,
    bytes: Vec<u8>,
) -> Result<LoadReport, String> {
    with_engine(&app, &state, |engine| engine.load(&bytes))
}

#[tauri::command]
pub fn play_key_sound(
    app: AppHandle,
    state: State<'_, KeyAudio>,
    volume: f64,
    ipc_ms: u64,
    request: u64,
) -> Result<PlayReport, String> {
    with_engine(&app, &state, |engine| engine.play(volume, ipc_ms, request))
}

fn with_engine<T>(
    app: &AppHandle,
    state: &KeyAudio,
    body: impl FnOnce(&Engine) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = state
        .inner
        .lock()
        .map_err(|_| "按键音状态锁定失败".to_owned())?;
    if state.broken.swap(false, Ordering::Relaxed) {
        // The output stream died (device unplugged or switched); rebuild it here.
        *guard = None;
    }
    if guard.is_none() {
        let logs = log_channel(app);
        *guard = Some(Engine::start(logs, Arc::clone(&state.broken))?);
    }
    let Some(engine) = guard.as_ref() else {
        return Err("按键音输出没有就绪".to_owned());
    };
    body(engine)
}

impl Engine {
    fn start(logs: &'static SyncSender<LogEvent>, broken: Arc<AtomicBool>) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "没有可用的音频输出".to_owned())?;
        let device_name = device
            .description()
            .map(|description| description.name().to_owned())
            .unwrap_or_else(|_| "unknown".to_owned());
        let supported = device
            .default_output_config()
            .map_err(|error| format!("读不到默认输出格式：{error}"))?;
        let format = supported.sample_format();
        let (buffer, requested) = preferred_buffer(*supported.buffer_size());
        let mut config = supported.config();
        config.buffer_size = buffer;
        let channels = config.channels;
        let rate = config.sample_rate;
        if channels == 0 || rate == 0 {
            return Err("输出设备的格式无效".to_owned());
        }

        let opened = match open_output(
            &device,
            config,
            format,
            usize::from(channels),
            rate,
            logs,
            &broken,
        ) {
            Ok(opened) => (opened, false),
            Err(first) => {
                let mut fallback = config;
                fallback.buffer_size = cpal::BufferSize::Default;
                match open_output(
                    &device,
                    fallback,
                    format,
                    usize::from(channels),
                    rate,
                    logs,
                    &broken,
                ) {
                    Ok(opened) => (opened, true),
                    Err(second) => {
                        return Err(format!(
                            "按键音输出打不开：{first}；默认缓冲也失败：{second}"
                        ));
                    }
                }
            }
        };
        let ((commands, stream), used_default) = opened;
        let actual = stream.buffer_size().unwrap_or(requested);
        stream
            .play()
            .map_err(|error| format!("按键音输出无法开始：{error}"))?;
        let _ = logs.try_send(LogEvent::Line(event_line(&serde_json::json!({
            "event": "device",
            "host": host.id().name(),
            "device": device_name,
            "rate": rate,
            "channels": channels,
            "format": format.to_string(),
            "requestedFrames": requested,
            "bufferFrames": actual,
            "bufferFallback": used_default,
        }))));

        Ok(Self {
            commands,
            logs,
            clip: Mutex::new(None),
            next_voice: AtomicU64::new(0),
            rate,
            channels,
            buffer_frames: actual,
            _stream: stream,
        })
    }

    fn load(&self, bytes: &[u8]) -> Result<LoadReport, String> {
        let prepared = prepare_clip(bytes, self.rate)?;
        let mut clip = self
            .clip
            .lock()
            .map_err(|_| "按键音片段锁定失败".to_owned())?;
        *clip = Some(Arc::clone(&prepared.samples));
        drop(clip);
        let _ = self.logs.try_send(LogEvent::Line(event_line(&serde_json::json!({
            "event": "loaded",
            "frames": prepared.frames,
            "rate": prepared.source_rate,
            "deviceRate": self.rate,
            "makeup": prepared.makeup,
            "bufferFrames": self.buffer_frames,
        }))));
        Ok(LoadReport {
            frames: prepared.frames,
            rate: prepared.source_rate,
            device_rate: self.rate,
            channels: self.channels,
            makeup: prepared.makeup,
            buffer_frames: self.buffer_frames,
        })
    }

    fn play(&self, volume: f64, ipc_ms: u64, request: u64) -> Result<PlayReport, String> {
        if !volume.is_finite() || volume <= 0.0 {
            return Ok(PlayReport { voice: 0 });
        }
        let gain = ((volume / 100.0).clamp(0.0, 1.0)) as f32;
        let samples = {
            let clip = self
                .clip
                .lock()
                .map_err(|_| "按键音片段锁定失败".to_owned())?;
            let Some(samples) = clip.as_ref() else {
                return Err("not-ready".to_owned());
            };
            Arc::clone(samples)
        };
        let id = self.next_voice.fetch_add(1, Ordering::Relaxed) + 1;
        let command = PlayCommand {
            samples,
            gain,
            id,
            request,
            enqueued: Instant::now(),
            ipc_ms,
        };
        match self.commands.try_send(command) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => return Err("queue-full".to_owned()),
            Err(TrySendError::Disconnected(_)) => return Err("mixer-closed".to_owned()),
        }
        let _ = self.logs.try_send(LogEvent::Queued {
            voice: id,
            request,
            gain,
            ipc_ms,
        });
        Ok(PlayReport { voice: id })
    }
}

fn preferred_buffer(supported: SupportedBufferSize) -> (cpal::BufferSize, u32) {
    match supported {
        SupportedBufferSize::Range { min, max } => {
            let frames = PREFERRED_FRAMES.clamp(min, max);
            (cpal::BufferSize::Fixed(frames), frames)
        }
        SupportedBufferSize::Unknown => (cpal::BufferSize::Default, 0),
    }
}

fn open_output(
    device: &cpal::Device,
    config: StreamConfig,
    format: SampleFormat,
    channels: usize,
    rate: u32,
    logs: &SyncSender<LogEvent>,
    broken: &Arc<AtomicBool>,
) -> Result<(SyncSender<PlayCommand>, cpal::Stream), String> {
    match format {
        SampleFormat::F32 => build_stream::<f32>(device, config, channels, rate, logs, broken),
        SampleFormat::I16 => build_stream::<i16>(device, config, channels, rate, logs, broken),
        SampleFormat::U16 => build_stream::<u16>(device, config, channels, rate, logs, broken),
        SampleFormat::F64 => build_stream::<f64>(device, config, channels, rate, logs, broken),
        other => Err(format!("不支持的输出格式 {other}")),
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    channels: usize,
    _rate: u32,
    logs: &SyncSender<LogEvent>,
    broken: &Arc<AtomicBool>,
) -> Result<(SyncSender<PlayCommand>, cpal::Stream), String>
where
    T: SizedSample + FromSample<f32>,
{
    let (commands, rx) = sync_channel(128);
    let mut callback = AudioCallback {
        rx,
        voices: Vec::with_capacity(MAX_VOICES),
        logs: logs.clone(),
        channels,
        scratch: Vec::new(),
        clock_logged: false,
    };
    let error_logs = logs.clone();
    let error_broken = Arc::clone(broken);
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], info| callback.render(data, output_delay_us(info)),
            move |error| {
                // Mark the engine for a rebuild on the next key; without this a
                // dead stream would keep the cached engine silent forever.
                error_broken.store(true, Ordering::Relaxed);
                let _ = error_logs.try_send(LogEvent::Stream(error.to_string()));
            },
            Some(Duration::from_secs(2)),
        )
        .map_err(|error| error.to_string())?;
    Ok((commands, stream))
}

fn output_delay_us(info: &cpal::OutputCallbackInfo) -> Option<u64> {
    let timestamp = info.timestamp();
    let delay = timestamp
        .playback
        .checked_duration_since(timestamp.callback)?;
    u64::try_from(delay.as_micros()).ok()
}

impl AudioCallback {
    fn render<T>(&mut self, data: &mut [T], playback_us: Option<u64>)
    where
        T: cpal::Sample + FromSample<f32>,
    {
        let mut scratch = std::mem::take(&mut self.scratch);
        if scratch.len() < data.len() {
            scratch.resize(data.len(), 0.0);
        }
        self.mix(&mut scratch[..data.len()], playback_us);
        for (destination, source) in data.iter_mut().zip(scratch.iter().copied()) {
            *destination = T::from_sample(source);
        }
        self.scratch = scratch;
    }

    fn mix(&mut self, output: &mut [f32], playback_us: Option<u64>) {
        let channels = self.channels.max(1);
        let frames = output.len() / channels;
        if !self.clock_logged {
            self.clock_logged = true;
            let _ = self.logs.try_send(LogEvent::Clock {
                playback_us,
                frames,
            });
        }
        output.fill(0.0);
        self.accept_voices();
        if frames == 0 {
            return;
        }
        let mut index = 0;
        while index < self.voices.len() {
            let done = self.render_voice(index, output, channels, frames, playback_us);
            if done {
                self.voices.swap_remove(index);
            } else {
                index += 1;
            }
        }
        for sample in output.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }

    fn accept_voices(&mut self) {
        while let Ok(command) = self.rx.try_recv() {
            if self.voices.len() >= MAX_VOICES {
                let _ = self.logs.try_send(LogEvent::Skip {
                    id: command.id,
                    request: command.request,
                });
                continue;
            }
            self.voices.push(Voice {
                samples: command.samples,
                cursor: 0,
                gain: command.gain,
                id: command.id,
                request: command.request,
                enqueued: command.enqueued,
                ipc_ms: command.ipc_ms,
                heard: false,
            });
        }
    }

    fn render_voice(
        &mut self,
        index: usize,
        output: &mut [f32],
        channels: usize,
        frames: usize,
        playback_us: Option<u64>,
    ) -> bool {
        let first = {
            let voice = &mut self.voices[index];
            let first = !voice.heard;
            voice.heard = true;
            first
        };
        if first {
            let voice = &self.voices[index];
            let queue_us = elapsed_us(voice.enqueued);
            let event = LogEvent::Mixed {
                id: voice.id,
                request: voice.request,
                queue_us,
                playback_us,
                ipc_ms: voice.ipc_ms,
                active: self.voices.len(),
            };
            let _ = self.logs.try_send(event);
        }

        let voice = &mut self.voices[index];
        let gain = voice.gain;
        let mut cursor = voice.cursor;
        let last = voice.samples.len();
        for frame in 0..frames {
            if cursor >= last {
                break;
            }
            let value = voice.samples[cursor] * gain;
            let base = frame * channels;
            for channel in 0..channels {
                output[base + channel] += value;
            }
            cursor += 1;
        }
        voice.cursor = cursor;
        let finished = cursor >= voice.samples.len();
        if finished {
            let _ = self.logs.try_send(LogEvent::Finished {
                id: voice.id,
                request: voice.request,
                age_ms: elapsed_ms(voice.enqueued),
            });
        }
        finished
    }
}

fn elapsed_us(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_micros()).unwrap_or(u64::MAX)
}

fn elapsed_ms(since: Instant) -> u64 {
    u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn iso_now() -> String {
    let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return "1970-01-01T00:00:00.000Z".to_owned();
    };
    format_unix(duration.as_secs(), duration.subsec_millis())
}

fn format_unix(secs: u64, millis: u32) -> String {
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let tod = secs % 86_400;
    let (year, month, day) = civil_from_unix_days(days);
    let hour = tod / 3_600;
    let minute = (tod % 3_600) / 60;
    let second = tod % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

fn civil_from_unix_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = u64::try_from(z - era * 146_097).unwrap_or(0);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year_base = i64::try_from(yoe).unwrap_or(0) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_base + i64::from(month <= 2);
    (
        i32::try_from(year).unwrap_or(0),
        u32::try_from(month).unwrap_or(0),
        u32::try_from(day).unwrap_or(0),
    )
}

fn prepare_clip(bytes: &[u8], device_rate: u32) -> Result<Prepared, String> {
    if device_rate == 0 {
        return Err("输出采样率无效".to_owned());
    }
    let decoded = decode_wav(bytes)?;
    let mono = to_mono(decoded)?;
    if mono.samples.is_empty() {
        return Err("按键音是空的".to_owned());
    }
    let resampled = resample(&mono.samples, mono.rate, device_rate)?;
    let peak = peak_of(&resampled);
    let makeup = makeup_gain(peak);
    let samples: Arc<[f32]> = resampled
        .into_iter()
        .map(|sample| sample * makeup)
        .collect::<Vec<_>>()
        .into();
    let frames = samples.len();
    Ok(Prepared {
        samples,
        source_rate: mono.rate,
        frames,
        makeup,
    })
}

fn makeup_gain(peak: f32) -> f32 {
    if !peak.is_finite() || peak <= 0.000_01 {
        return 1.0;
    }
    let target = 10f32.powf(TARGET_DB / 20.0);
    (target / peak).min(MAX_MAKEUP)
}

fn peak_of(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

struct Decoded {
    rate: u32,
    channels: u16,
    samples: Vec<f32>,
}

fn decode_wav(bytes: &[u8]) -> Result<Decoded, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("按键音不是 WAV".to_owned());
    }
    let mut offset = 12usize;
    let mut format = 0u16;
    let mut channels = 0u16;
    let mut rate = 0u32;
    let mut bits = 0u16;
    let mut data: Option<&[u8]> = None;
    while offset + 8 <= bytes.len() {
        let id = bytes.get(offset..offset + 4).ok_or("按键音文件损坏")?;
        let size = usize::try_from(read_u32(bytes, offset + 4)?).map_err(|_| "按键音文件损坏")?;
        let start = offset + 8;
        let end = start.checked_add(size).ok_or("按键音文件损坏")?;
        if end > bytes.len() {
            return Err("按键音文件不完整".to_owned());
        }
        if id == b"fmt " && size >= 16 {
            format = read_u16(bytes, start)?;
            channels = read_u16(bytes, start + 2)?;
            rate = read_u32(bytes, start + 4)?;
            bits = read_u16(bytes, start + 14)?;
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
            break;
        }
        let padded = size + (size & 1);
        offset = start.checked_add(padded).ok_or("按键音文件损坏")?;
    }
    let Some(data) = data else {
        return Err("按键音没有音频数据".to_owned());
    };
    if data.len() > 8 * 1024 * 1024 {
        return Err("按键音文件太大".to_owned());
    }
    if channels == 0 || rate == 0 {
        return Err("按键音格式无效".to_owned());
    }
    let samples = match (format, bits) {
        (1, 16) => decode_pcm16(data)?,
        (3, 32) => decode_f32(data)?,
        _ => return Err("只支持 16 位 PCM 或 32 位浮点 WAV".to_owned()),
    };
    let channel_count = usize::from(channels);
    if samples.len() % channel_count != 0 {
        return Err("按键音声道数据不完整".to_owned());
    }
    let frames = samples.len() / channel_count;
    if frames > (rate as usize).saturating_mul(2) {
        return Err("按键音太长".to_owned());
    }
    Ok(Decoded {
        rate,
        channels,
        samples,
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let chunk = bytes.get(offset..offset + 2).ok_or("按键音文件损坏")?;
    let array: [u8; 2] = chunk.try_into().map_err(|_| "按键音文件损坏")?;
    Ok(u16::from_le_bytes(array))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let chunk = bytes.get(offset..offset + 4).ok_or("按键音文件损坏")?;
    let array: [u8; 4] = chunk.try_into().map_err(|_| "按键音文件损坏")?;
    Ok(u32::from_le_bytes(array))
}

fn decode_pcm16(data: &[u8]) -> Result<Vec<f32>, String> {
    let (chunks, rest) = data.as_chunks::<2>();
    if !rest.is_empty() {
        return Err("按键音数据不完整".to_owned());
    }
    let mut samples = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        samples.push(f32::from(i16::from_le_bytes(*chunk)) / 32_768.0);
    }
    Ok(samples)
}

fn decode_f32(data: &[u8]) -> Result<Vec<f32>, String> {
    let (chunks, rest) = data.as_chunks::<4>();
    if !rest.is_empty() {
        return Err("按键音数据不完整".to_owned());
    }
    let mut samples = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        let value = f32::from_le_bytes(*chunk);
        samples.push(if value.is_finite() { value } else { 0.0 });
    }
    Ok(samples)
}

fn to_mono(decoded: Decoded) -> Result<Decoded, String> {
    if decoded.channels == 1 {
        return Ok(decoded);
    }
    let channels = usize::from(decoded.channels);
    let frames = decoded.samples.len() / channels;
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        let start = frame * channels;
        let sum: f32 = decoded.samples[start..start + channels]
            .iter()
            .copied()
            .sum();
        mono.push(sum / f32::from(decoded.channels));
    }
    Ok(Decoded {
        rate: decoded.rate,
        channels: 1,
        samples: mono,
    })
}

fn resample(input: &[f32], from: u32, to: u32) -> Result<Vec<f32>, String> {
    if from == 0 || to == 0 {
        return Err("采样率无效".to_owned());
    }
    if from == to {
        return Ok(input.to_vec());
    }
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let out_len =
        usize::try_from(u64::from(to) * u64::try_from(input.len()).unwrap_or(0) / u64::from(from))
            .map_err(|_| "按键音太长")?;
    let mut output = Vec::with_capacity(out_len);
    let scale = f64::from(from) / f64::from(to);
    let last = input.len() - 1;
    for index in 0..out_len {
        let position = index as f64 * scale;
        let left = usize::try_from(position.floor() as u64)
            .unwrap_or(last)
            .min(last);
        let right = (left + 1).min(last);
        let fraction = (position - left as f64) as f32;
        let start = input[left];
        let end = input[right];
        output.push(start + (end - start) * fraction);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn quiet_recording_is_normalized_to_about_minus_one_db() {
        let peak = 10f32.powf(-21.2 / 20.0);
        let amplified = peak * makeup_gain(peak);
        let db = 20.0 * amplified.log10();
        assert!((db - TARGET_DB).abs() < 0.05, "{db}");
    }

    #[test]
    fn loud_recording_is_not_amplified() {
        assert!(makeup_gain(0.9) < 1.0);
    }

    #[test]
    fn silence_does_not_receive_extreme_gain() {
        assert_eq!(makeup_gain(0.0), 1.0);
        assert!(makeup_gain(10f32.powf(-60.0 / 20.0)) <= MAX_MAKEUP);
    }

    #[test]
    fn unix_timestamps_format_as_utc() {
        assert_eq!(format_unix(0, 0), "1970-01-01T00:00:00.000Z");
        assert_eq!(format_unix(1_704_067_200, 125), "2024-01-01T00:00:00.125Z");
    }

    #[test]
    fn stereo_pcm_averages_to_mono() {
        let wav = pcm16_wav(2, 48_000, &[16_384, 0, 0, 16_384]);
        let prepared = prepare_clip(&wav, 48_000).expect("pcm wav");
        assert_eq!(prepared.frames, 2);
        assert!((prepared.samples[0] - prepared.samples[1]).abs() < 0.000_1);
    }

    #[test]
    fn default_pack_peaks_near_minus_one_db() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../public/sounds/key-sound/Default.wav");
        let bytes = std::fs::read(path).expect("default click");
        let prepared = prepare_clip(&bytes, 48_000).expect("default wav");
        let duration_ms = prepared.frames * 1_000 / 48_000;
        assert!((70..120).contains(&duration_ms), "{duration_ms}");
        let db = 20.0 * peak_of(&prepared.samples).log10();
        assert!((db - TARGET_DB).abs() < 0.2, "{db}");
    }

    #[test]
    fn resample_preserves_duration() {
        let input = [0.0, 1.0, 0.0, -1.0];
        assert_eq!(resample(&input, 48_000, 48_000).expect("same rate"), input);
        assert_eq!(
            resample(&input, 48_000, 24_000).expect("half rate").len(),
            2
        );
    }

    #[test]
    fn overlapping_voices_add_and_finish_without_cutting() {
        let (mut callback, commands, _logs) = test_callback();
        commands
            .send(play_command(1, vec![0.2, 0.2]))
            .expect("queue first");
        commands
            .send(play_command(2, vec![0.3]))
            .expect("queue second");
        commands
            .send(play_command(3, vec![0.4, 0.4]))
            .expect("queue third");
        let mut output = [0.0];
        callback.mix(&mut output, Some(1_000));
        assert!((output[0] - 0.9).abs() < 0.000_1, "{}", output[0]);
        assert_eq!(callback.voices.len(), 2);

        let mut output = [0.0];
        callback.mix(&mut output, Some(1_000));
        assert!((output[0] - 0.6).abs() < 0.000_1, "{}", output[0]);
        assert!(callback.voices.is_empty());
    }

    #[test]
    fn overlap_above_full_scale_is_limited_without_lowering_a_single_voice() {
        let (mut callback, commands, _logs) = test_callback();
        commands.send(play_command(1, vec![0.8])).expect("first");
        let mut output = [0.0];
        callback.mix(&mut output, None);
        assert!((output[0] - 0.8).abs() < 0.000_1);

        commands.send(play_command(2, vec![0.8])).expect("second");
        commands.send(play_command(3, vec![0.8])).expect("third");
        let mut output = [0.0];
        callback.mix(&mut output, None);
        assert!((output[0] - 1.0).abs() < 0.000_1);
    }

    fn test_callback() -> (AudioCallback, SyncSender<PlayCommand>, Receiver<LogEvent>) {
        let (commands, rx) = sync_channel(8);
        let (logs, log_rx) = sync_channel(16);
        let callback = AudioCallback {
            rx,
            voices: Vec::with_capacity(MAX_VOICES),
            logs,
            channels: 1,
            scratch: Vec::new(),
            clock_logged: false,
        };
        (callback, commands, log_rx)
    }

    fn play_command(id: u64, samples: Vec<f32>) -> PlayCommand {
        PlayCommand {
            samples: samples.into(),
            gain: 1.0,
            id,
            request: id,
            enqueued: Instant::now(),
            ipc_ms: 0,
        }
    }

    fn pcm16_wav(channels: u16, rate: u32, samples: &[i16]) -> Vec<u8> {
        let data_bytes = samples.len() * 2;
        let mut bytes = Vec::new();
        bytes.extend(b"RIFF");
        bytes.extend(u32::try_from(36 + data_bytes).unwrap_or(0).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes());
        bytes.extend(channels.to_le_bytes());
        bytes.extend(rate.to_le_bytes());
        let block = u32::from(channels) * 2;
        bytes.extend((rate * block).to_le_bytes());
        bytes.extend(u16::try_from(block).unwrap_or(0).to_le_bytes());
        bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data");
        bytes.extend(u32::try_from(data_bytes).unwrap_or(0).to_le_bytes());
        for sample in samples {
            bytes.extend(sample.to_le_bytes());
        }
        bytes
    }
}
