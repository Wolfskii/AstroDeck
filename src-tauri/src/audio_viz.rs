use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const EVENT_NAME: &str = "os-audio-viz";
const BAND_COUNT: usize = 32;
const FFT_SIZE: usize = 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioVizFrame {
    pub active: bool,
    pub beat: f32,
    pub rms: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub bands: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl AudioVizFrame {
    fn inactive(error: Option<String>) -> Self {
        Self {
            active: false,
            beat: 0.0,
            rms: 0.0,
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            bands: vec![0.0; BAND_COUNT],
            error,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioVisualizerStatus {
    pub supported: bool,
    pub enabled: bool,
    pub running: bool,
    pub error: Option<String>,
}

struct CaptureCtl {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    running: bool,
    error: Option<String>,
}

impl CaptureCtl {
    fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
            running: false,
            error: None,
        }
    }
}

static CAPTURE: Mutex<Option<CaptureCtl>> = Mutex::new(None);

pub fn supported() -> bool {
    cfg!(windows)
}

fn emit_frame(app: &AppHandle, frame: &AudioVizFrame) {
    if let Err(err) = app.emit(EVENT_NAME, frame) {
        log::debug!("Failed to emit {EVENT_NAME}: {err}");
    }
}

pub fn sync(app: &AppHandle, enabled: bool) {
    if enabled && supported() {
        start(app.clone());
    } else {
        stop(app);
        if enabled && !supported() {
            emit_frame(
                app,
                &AudioVizFrame::inactive(Some(
                    "System audio capture is currently available on Windows.".into(),
                )),
            );
        }
    }
}

fn start(app: AppHandle) {
    let mut slot = match CAPTURE.lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };
    let ctl = slot.get_or_insert_with(CaptureCtl::new);
    if ctl.running {
        return;
    }
    ctl.stop.store(false, Ordering::SeqCst);
    ctl.error = None;
    ctl.running = true;
    let stop = ctl.stop.clone();
    ctl.thread = thread::Builder::new()
        .name("os-audio-viz".into())
        .spawn(move || capture_thread(app, stop))
        .ok();
}

fn stop(app: &AppHandle) {
    let thread = {
        let mut slot = match CAPTURE.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if let Some(ctl) = slot.as_mut() {
            ctl.stop.store(true, Ordering::SeqCst);
            ctl.running = false;
            ctl.error = None;
            ctl.thread.take()
        } else {
            None
        }
    };
    if let Some(handle) = thread {
        let _ = handle.join();
    }
    emit_frame(app, &AudioVizFrame::inactive(None));
}

pub fn status(enabled: bool) -> AudioVisualizerStatus {
    let (running, error) = CAPTURE
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().map(|ctl| (ctl.running, ctl.error.clone())))
        .unwrap_or((false, None));
    AudioVisualizerStatus {
        supported: supported(),
        enabled,
        running,
        error,
    }
}

fn set_error(message: String) {
    if let Ok(mut slot) = CAPTURE.lock() {
        if let Some(ctl) = slot.as_mut() {
            ctl.error = Some(message);
            ctl.running = false;
        }
    }
}

fn capture_thread(app: AppHandle, stop: Arc<AtomicBool>) {
    #[cfg(windows)]
    {
        while !stop.load(Ordering::SeqCst) {
            match windows_loop(&app, &stop) {
                Ok(()) => break,
                Err(err) => {
                    log::warn!("OS audio visualizer: {err}");
                    set_error(err.clone());
                    emit_frame(&app, &AudioVizFrame::inactive(Some(err)));
                    for _ in 0..20 {
                        if stop.load(Ordering::SeqCst) {
                            return;
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        let _ = (&app, stop);
    }
}

#[cfg(windows)]
fn windows_loop(app: &AppHandle, stop: &AtomicBool) -> Result<(), String> {
    use rustfft::num_complex::Complex;
    use rustfft::FftPlanner;
    use std::collections::VecDeque;
    use wasapi::*;

    if !initialize_mta().is_ok() {
        return Err("Could not initialize COM for speaker capture.".into());
    }
    let device = get_default_device(&Direction::Render).map_err(|err| err.to_string())?;
    let mut audio_client = device.get_iaudioclient().map_err(|err| err.to_string())?;
    let desired_format = WaveFormat::new(32, 32, &SampleType::Float, 44100, 2, None);
    let channels = 2_usize;
    let (_def_time, min_time) = audio_client
        .get_device_period()
        .map_err(|err| err.to_string())?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_time,
    };
    audio_client
        .initialize_client(&desired_format, &Direction::Capture, &mode)
        .map_err(|err| err.to_string())?;

    let event = audio_client
        .set_get_eventhandle()
        .map_err(|err| err.to_string())?;
    let capture = audio_client
        .get_audiocaptureclient()
        .map_err(|err| err.to_string())?;
    audio_client.start_stream().map_err(|err| err.to_string())?;

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let window: Vec<f32> = (0..FFT_SIZE)
        .map(|i| {
            0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FFT_SIZE as f32).cos()
        })
        .collect();
    let mut fft_buf = vec![Complex::new(0.0, 0.0); FFT_SIZE];
    let mut bytes: VecDeque<u8> = VecDeque::with_capacity(FFT_SIZE * 16);
    let mut samples: Vec<f32> = Vec::with_capacity(FFT_SIZE * 2);
    let mut bands_raw = [0.0_f32; BAND_COUNT];
    let mut bands_ema = [0.0_f32; BAND_COUNT];
    let mut energy_fast = 0.0_f32;
    let mut energy_slow = 0.02_f32;
    let mut beat_hold = 0.0_f32;
    let mut last_emit = Instant::now() - Duration::from_millis(50);
    let sample_rate = 44100.0_f32;

    while !stop.load(Ordering::SeqCst) {
        if event.wait_for_event(200).is_err() {
            continue;
        }
        capture
            .read_from_device_to_deque(&mut bytes)
            .map_err(|err| err.to_string())?;

        let frame_bytes = channels * 4;
        while bytes.len() >= frame_bytes {
            let mut left = [0u8; 4];
            let mut right = [0u8; 4];
            for slot in &mut left {
                *slot = bytes.pop_front().unwrap_or(0);
            }
            for slot in &mut right {
                *slot = bytes.pop_front().unwrap_or(0);
            }
            let l = f32::from_le_bytes(left);
            let r = f32::from_le_bytes(right);
            samples.push((l + r) * 0.5);
        }
        if samples.len() > FFT_SIZE * 3 {
            let extra = samples.len() - FFT_SIZE * 2;
            samples.drain(0..extra);
        }

        while samples.len() >= FFT_SIZE {
            let hop = FFT_SIZE / 2;
            let mut sum_sq = 0.0_f32;
            for i in 0..FFT_SIZE {
                let sample = samples[i];
                sum_sq += sample * sample;
                fft_buf[i] = Complex::new(sample * window[i], 0.0);
            }
            fft.process(&mut fft_buf);
            samples.drain(0..hop);
            let rms = (sum_sq / FFT_SIZE as f32).sqrt().min(1.0);

            bands_raw.fill(0.0);
            let mut counts = [0.0_f32; BAND_COUNT];
            let nyquist = sample_rate * 0.5;
            let bins = FFT_SIZE / 2;

            for bin in 1..bins {
                let freq = bin as f32 * sample_rate / FFT_SIZE as f32;
                if freq >= nyquist {
                    break;
                }
                let mag = fft_buf[bin].norm() / (bins as f32);
                let min_f = 40.0_f32;
                let max_f = 16_000.0_f32;
                let t = ((freq.max(min_f).ln() - min_f.ln()) / (max_f.ln() - min_f.ln()))
                    .clamp(0.0, 1.0);
                let index = (t * (BAND_COUNT as f32 - 1.0)).round() as usize;
                bands_raw[index] += mag;
                counts[index] += 1.0;
            }
            for i in 0..BAND_COUNT {
                let mean = if counts[i] > 0.0 {
                    bands_raw[i] / counts[i]
                } else {
                    0.0
                };
                let boosted = (mean * 18.0).min(1.0).powf(0.55);
                bands_ema[i] = bands_ema[i] * 0.55 + boosted * 0.45;
            }

            let bass: f32 = bands_ema[0..6].iter().sum::<f32>() / 6.0;
            let mid: f32 = bands_ema[6..18].iter().sum::<f32>() / 12.0;
            let treble: f32 = bands_ema[18..].iter().sum::<f32>() / 14.0;

            energy_fast = energy_fast * 0.62 + rms * 0.38;
            energy_slow = energy_slow * 0.97 + rms * 0.03;
            let onset = energy_fast > (energy_slow * 1.42).max(0.028) && bass > 0.08;
            if onset {
                beat_hold = 1.0;
            } else {
                beat_hold *= 0.84;
            }
            let beat = (beat_hold * 0.72 + energy_fast.min(0.6) * 0.9 + bass * 0.35).clamp(0.0, 1.0);

            if last_emit.elapsed() >= Duration::from_millis(33) {
                last_emit = Instant::now();
                emit_frame(
                    app,
                    &AudioVizFrame {
                        active: true,
                        beat,
                        rms,
                        bass,
                        mid,
                        treble,
                        bands: bands_ema.to_vec(),
                        error: None,
                    },
                );
                if let Ok(mut slot) = CAPTURE.lock() {
                    if let Some(ctl) = slot.as_mut() {
                        ctl.running = true;
                        ctl.error = None;
                    }
                }
            }
        }
    }

    let _ = audio_client.stop_stream();
    Ok(())
}
