use std::fs;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::cli;
use crate::world::TICK_RATE;

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:9321";
pub const CONNECT_GAP: Duration = Duration::from_micros(500);
pub const IDLE_SLEEP: Duration = Duration::from_micros(500);
pub const TAG_COUNT: usize = 512;

const DEFAULT_BOTS: usize = 100;
const DEFAULT_MEASURED_BOTS: usize = 100;
const DEFAULT_SECONDS: f32 = 20.0;
const DEFAULT_WARMUP: f32 = 5.0;
const DEFAULT_BOTS_PER_THREAD: usize = 64;
const CONNECT_SLACK: Duration = Duration::from_secs(2);
const SAMPLE_INTERVAL: Duration = Duration::from_millis(200);
const CLOCK_TICKS_PER_SECOND: f64 = 100.0;
const PAGE_SIZE: u64 = 4096;
const TAG_BASE: f32 = -0.25;
const TAG_STEP: f32 = 0.001;

pub struct Options {
    pub address: String,
    pub bots: usize,
    pub measure_stride: usize,
    pub threads: usize,
    pub seconds: Duration,
    pub warmup: Duration,
    pub input_interval: Duration,
    pub input_hz: f32,
    pub server_pid: Option<u32>,
    pub json: Option<String>,
}

impl Options {
    pub fn from_cli() -> Options {
        let bots = cli::flag("--bots")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_BOTS)
            .max(1);
        let measured_bots = cli::flag("--measure-bots")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(DEFAULT_MEASURED_BOTS)
            .min(bots)
            .max(1);
        let threads = cli::flag("--threads")
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or_else(|| bots.div_ceil(DEFAULT_BOTS_PER_THREAD))
            .clamp(1, bots);
        let input_hz = cli::flag("--input-hz")
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(f32::from(TICK_RATE))
            .max(1.0);

        Options {
            address: cli::flag("--addr").unwrap_or_else(|| DEFAULT_ADDRESS.to_owned()),
            bots,
            measure_stride: (bots / measured_bots).max(1),
            threads,
            seconds: Duration::from_secs_f32(
                cli::flag("--seconds")
                    .and_then(|value| value.parse::<f32>().ok())
                    .unwrap_or(DEFAULT_SECONDS),
            ),
            warmup: Duration::from_secs_f32(
                cli::flag("--warmup")
                    .and_then(|value| value.parse::<f32>().ok())
                    .unwrap_or(DEFAULT_WARMUP),
            ),
            input_interval: Duration::from_secs_f32(1.0 / input_hz),
            input_hz,
            server_pid: cli::flag("--server-pid").and_then(|value| value.parse::<u32>().ok()),
            json: cli::flag("--json"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Window {
    pub measure_start: Instant,
    pub measure_end: Instant,
}

impl Window {
    pub fn measuring(&self, now: Instant) -> bool {
        now >= self.measure_start && now < self.measure_end
    }
}

#[derive(Default)]
pub struct Stats {
    pub measured: bool,
    pub joined: bool,
    pub lost: Option<String>,
    pub snapshots: u64,
    pub snapshot_bytes: u64,
    pub inputs: u64,
    pub congested: u64,
    pub rtt_micros: Vec<u32>,
}

impl Stats {
    pub fn measured(measured: bool) -> Stats {
        Stats {
            measured,
            ..Stats::default()
        }
    }
}

pub fn tag_pitch(index: usize) -> f32 {
    TAG_BASE + index as f32 * TAG_STEP
}

pub fn tag_index(pitch: f32) -> Option<usize> {
    let raw = ((pitch - TAG_BASE) / TAG_STEP).round();
    if raw < 0.0 || raw >= TAG_COUNT as f32 {
        return None;
    }
    let index = raw as usize;
    (tag_pitch(index).to_bits() == pitch.to_bits()).then_some(index)
}

pub fn circling_input(elapsed: f32, phase: f32) -> (f32, f32, f32) {
    let angle = elapsed + phase;
    let (move_x, move_z) = (angle.cos(), angle.sin());
    (move_x, move_z, (-move_x).atan2(-move_z))
}

pub trait LoadBot {
    fn pump(&mut self, now: Instant, window: &Window, input_interval: Duration);
    fn into_stats(self) -> Stats;
}

pub trait BotFactory: Sync {
    type Bot: LoadBot;

    fn connect(&self, index: usize, measured: bool, now: Instant) -> Result<Self::Bot, String>;
}

pub struct Run {
    pub window: Window,
    pub stats: Vec<Stats>,
    pub server_samples: Vec<ProcessSample>,
    pub client_samples: Vec<ProcessSample>,
}

pub fn run<F: BotFactory>(stack: &str, options: &Options, factory: &F) -> Run {
    let connect_budget = CONNECT_SLACK + CONNECT_GAP * (options.bots.div_ceil(options.threads) as u32);
    let start = Instant::now();
    let window = Window {
        measure_start: start + connect_budget + options.warmup,
        measure_end: start + connect_budget + options.warmup + options.seconds,
    };

    println!(
        "bench: {stack} · {} bots (every {} measured) on {} threads, input {:.0} Hz, warmup {:.0}s, measure {:.0}s",
        options.bots,
        options.measure_stride,
        options.threads,
        options.input_hz,
        options.warmup.as_secs_f32(),
        options.seconds.as_secs_f32()
    );

    let stop_sampling = AtomicBool::new(false);
    let mut stats = Vec::new();
    let mut server_samples = Vec::new();
    let mut client_samples = Vec::new();

    thread::scope(|scope| {
        let sampler = scope.spawn(|| {
            let mut server = Vec::new();
            let mut client = Vec::new();
            while !stop_sampling.load(Ordering::Relaxed) {
                if let Some(pid) = options.server_pid {
                    if let Some(sample) = read_process(&pid.to_string()) {
                        server.push(sample);
                    }
                }
                if let Some(sample) = read_process("self") {
                    client.push(sample);
                }
                thread::sleep(SAMPLE_INTERVAL);
            }
            (server, client)
        });

        let mut workers = Vec::new();
        let per_thread = options.bots.div_ceil(options.threads);
        let mut assigned = 0;
        while assigned < options.bots {
            let count = per_thread.min(options.bots - assigned);
            let first = assigned;
            workers.push(scope.spawn(move || run_shard(options, factory, window, first, count)));
            assigned += count;
        }

        for worker in workers {
            match worker.join() {
                Ok(shard) => stats.extend(shard),
                Err(_) => {
                    eprintln!("bench: a worker thread panicked");
                    exit(1);
                }
            }
        }

        stop_sampling.store(true, Ordering::Relaxed);
        if let Ok((server, client)) = sampler.join() {
            server_samples = server;
            client_samples = client;
        }
    });

    Run {
        window,
        stats,
        server_samples,
        client_samples,
    }
}

fn run_shard<F: BotFactory>(
    options: &Options,
    factory: &F,
    window: Window,
    first: usize,
    count: usize,
) -> Vec<Stats> {
    let mut bots = Vec::with_capacity(count);
    let mut refused = Vec::new();

    for slot in 0..count {
        let index = first + slot;
        match factory.connect(index, index % options.measure_stride == 0, Instant::now()) {
            Ok(bot) => bots.push(bot),
            Err(error) => refused.push(Stats {
                lost: Some(format!("cannot connect: {error}")),
                ..Stats::default()
            }),
        }
        for bot in bots.iter_mut() {
            bot.pump(Instant::now(), &window, options.input_interval);
        }
        thread::sleep(CONNECT_GAP);
    }

    while Instant::now() < window.measure_end {
        for bot in bots.iter_mut() {
            bot.pump(Instant::now(), &window, options.input_interval);
        }
        thread::sleep(IDLE_SLEEP);
    }

    let mut collected: Vec<Stats> = bots.into_iter().map(LoadBot::into_stats).collect();
    collected.append(&mut refused);
    collected
}

pub struct ProcessSample {
    at: Instant,
    cpu_seconds: f64,
    rss_bytes: u64,
}

fn read_process(pid: &str) -> Option<ProcessSample> {
    let at = Instant::now();
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after_name = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = after_name.split_whitespace().collect();
    let utime = fields.get(11)?.parse::<u64>().ok()?;
    let stime = fields.get(12)?.parse::<u64>().ok()?;
    let statm = fs::read_to_string(format!("/proc/{pid}/statm")).ok()?;
    let resident = statm.split_whitespace().nth(1)?.parse::<u64>().ok()?;
    Some(ProcessSample {
        at,
        cpu_seconds: (utime + stime) as f64 / CLOCK_TICKS_PER_SECOND,
        rss_bytes: resident * PAGE_SIZE,
    })
}

struct ResourceSummary {
    cpu_cores_mean: f64,
    cpu_cores_peak: f64,
    rss_mean_mb: f64,
    rss_peak_mb: f64,
}

fn summarize_resources(samples: &[ProcessSample], window: &Window) -> Option<ResourceSummary> {
    let inside: Vec<&ProcessSample> = samples
        .iter()
        .filter(|sample| sample.at >= window.measure_start && sample.at <= window.measure_end)
        .collect();
    let (first, last) = (inside.first()?, inside.last()?);
    let span = last.at.duration_since(first.at).as_secs_f64();
    if span <= 0.0 {
        return None;
    }

    let cpu_cores_peak = inside
        .windows(2)
        .map(|pair| {
            let seconds = pair[1].at.duration_since(pair[0].at).as_secs_f64();
            if seconds <= 0.0 {
                0.0
            } else {
                (pair[1].cpu_seconds - pair[0].cpu_seconds) / seconds
            }
        })
        .fold(0.0f64, f64::max);

    Some(ResourceSummary {
        cpu_cores_mean: (last.cpu_seconds - first.cpu_seconds) / span,
        cpu_cores_peak,
        rss_mean_mb: inside.iter().map(|sample| sample.rss_bytes as f64).sum::<f64>()
            / inside.len() as f64
            / (1024.0 * 1024.0),
        rss_peak_mb: inside.iter().map(|sample| sample.rss_bytes).max().unwrap_or(0) as f64
            / (1024.0 * 1024.0),
    })
}

fn percentile(sorted: &[u32], quantile: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let rank = (quantile * sorted.len() as f64).ceil() as usize;
    let index = rank.clamp(1, sorted.len()) - 1;
    sorted[index] as f64 / 1000.0
}

pub fn report(stack: &str, options: &Options, run: &Run) {
    let seconds = options.seconds.as_secs_f64();
    let stats = &run.stats;
    let measured_bots = stats.iter().filter(|stat| stat.measured).count();
    let joined = stats.iter().filter(|stat| stat.joined).count();
    let lost = stats.iter().filter(|stat| stat.lost.is_some()).count();
    let snapshots: u64 = stats.iter().map(|stat| stat.snapshots).sum();
    let snapshot_bytes: u64 = stats.iter().map(|stat| stat.snapshot_bytes).sum();
    let inputs: u64 = stats.iter().map(|stat| stat.inputs).sum();
    let congested: u64 = stats.iter().map(|stat| stat.congested).sum();

    let mut rtt: Vec<u32> = stats.iter().flat_map(|stat| stat.rtt_micros.iter().copied()).collect();
    rtt.sort_unstable();
    let rtt_mean = if rtt.is_empty() {
        f64::NAN
    } else {
        rtt.iter().map(|value| *value as f64).sum::<f64>() / rtt.len() as f64 / 1000.0
    };

    let snapshots_per_bot = if joined > 0 {
        snapshots as f64 / joined as f64 / seconds
    } else {
        0.0
    };
    let delivered = snapshots_per_bot / f64::from(TICK_RATE) * 100.0;
    let rate_of = |group: &dyn Fn(&&Stats) -> bool| {
        let members: Vec<&Stats> = stats.iter().filter(|stat| stat.joined && group(stat)).collect();
        if members.is_empty() {
            return f64::NAN;
        }
        members.iter().map(|stat| stat.snapshots).sum::<u64>() as f64 / members.len() as f64 / seconds
    };
    let decoding_rate = rate_of(&|stat| stat.measured);
    let counting_rate = rate_of(&|stat| !stat.measured);
    let mut per_bot: Vec<f64> = stats
        .iter()
        .filter(|stat| stat.joined)
        .map(|stat| stat.snapshots as f64 / seconds)
        .collect();
    per_bot.sort_by(f64::total_cmp);
    let spread = |quantile: f64| {
        if per_bot.is_empty() {
            return f64::NAN;
        }
        let rank = (quantile * per_bot.len() as f64).ceil() as usize;
        per_bot[rank.clamp(1, per_bot.len()) - 1]
    };
    let server = summarize_resources(&run.server_samples, &run.window);
    let client = summarize_resources(&run.client_samples, &run.window);
    let first_loss = stats.iter().find_map(|stat| stat.lost.clone());

    println!("stack           {stack}");
    println!("bots            {} requested, {joined} joined, {lost} lost", options.bots);
    if let Some(reason) = &first_loss {
        println!("first loss      {reason}");
    }
    println!(
        "latency         p50 {:.1} ms · p95 {:.1} ms · p99 {:.1} ms · max {:.1} ms · mean {rtt_mean:.1} ms ({} samples from {measured_bots} bots)",
        percentile(&rtt, 0.50),
        percentile(&rtt, 0.95),
        percentile(&rtt, 0.99),
        percentile(&rtt, 1.00),
        rtt.len()
    );
    println!(
        "snapshots       {:.0}/s total · {snapshots_per_bot:.1}/s per bot · {delivered:.0}% of {TICK_RATE} Hz",
        snapshots as f64 / seconds
    );
    let rate_label = |rate: f64| {
        if rate.is_nan() {
            "none".to_owned()
        } else {
            format!("{rate:.1}/s")
        }
    };
    println!(
        "per bot         min {:.1}/s · p50 {:.1}/s · max {:.1}/s · decoding bots {} · counting bots {}",
        spread(0.0),
        spread(0.50),
        spread(1.00),
        rate_label(decoding_rate),
        rate_label(counting_rate)
    );
    println!(
        "server egress   {:.1} MiB/s payload ({:.0} bytes per snapshot)",
        snapshot_bytes as f64 / seconds / (1024.0 * 1024.0),
        if snapshots > 0 { snapshot_bytes as f64 / snapshots as f64 } else { 0.0 }
    );
    println!(
        "inputs          {:.0}/s total{}",
        inputs as f64 / seconds,
        if congested > 0 {
            format!(" · {congested} dropped by client congestion")
        } else {
            String::new()
        }
    );
    match &server {
        Some(summary) => println!(
            "server process  cpu {:.2} cores mean · {:.2} peak · rss {:.0} MiB mean · {:.0} MiB peak",
            summary.cpu_cores_mean, summary.cpu_cores_peak, summary.rss_mean_mb, summary.rss_peak_mb
        ),
        None => println!("server process  not measured (pass --server-pid)"),
    }
    if let Some(summary) = &client {
        println!(
            "load generator  cpu {:.2} cores mean · rss {:.0} MiB peak",
            summary.cpu_cores_mean, summary.rss_peak_mb
        );
    }

    if let Some(path) = &options.json {
        let json = format!(
            concat!(
                "{{\n",
                "  \"stack\": \"{}\",\n",
                "  \"bots\": {},\n",
                "  \"measured_bots\": {},\n",
                "  \"joined\": {},\n",
                "  \"lost\": {},\n",
                "  \"seconds\": {:.1},\n",
                "  \"input_hz\": {:.1},\n",
                "  \"tick_rate\": {},\n",
                "  \"latency_ms\": {{ \"p50\": {:.2}, \"p95\": {:.2}, \"p99\": {:.2}, \"max\": {:.2}, \"mean\": {:.2}, \"samples\": {} }},\n",
                "  \"snapshots_per_second\": {:.1},\n",
                "  \"snapshots_per_second_per_bot\": {:.2},\n",
                "  \"delivered_percent\": {:.1},\n",
                "  \"per_bot_min\": {:.2},\n",
                "  \"per_bot_median\": {:.2},\n",
                "  \"per_bot_max\": {:.2},\n",
                "  \"decoding_bots_per_second\": {:.2},\n",
                "  \"counting_bots_per_second\": {:.2},\n",
                "  \"payload_mib_per_second\": {:.2},\n",
                "  \"snapshot_bytes\": {:.0},\n",
                "  \"inputs_per_second\": {:.1},\n",
                "  \"client_congested\": {},\n",
                "  \"server_cpu_cores_mean\": {:.3},\n",
                "  \"server_cpu_cores_peak\": {:.3},\n",
                "  \"server_rss_mib_mean\": {:.1},\n",
                "  \"server_rss_mib_peak\": {:.1},\n",
                "  \"generator_cpu_cores_mean\": {:.3},\n",
                "  \"generator_rss_mib_peak\": {:.1}\n",
                "}}\n"
            ),
            stack,
            options.bots,
            measured_bots,
            joined,
            lost,
            seconds,
            options.input_hz,
            TICK_RATE,
            percentile(&rtt, 0.50),
            percentile(&rtt, 0.95),
            percentile(&rtt, 0.99),
            percentile(&rtt, 1.00),
            rtt_mean,
            rtt.len(),
            snapshots as f64 / seconds,
            snapshots_per_bot,
            delivered,
            spread(0.0),
            spread(0.50),
            spread(1.00),
            decoding_rate,
            counting_rate,
            snapshot_bytes as f64 / seconds / (1024.0 * 1024.0),
            if snapshots > 0 { snapshot_bytes as f64 / snapshots as f64 } else { 0.0 },
            inputs as f64 / seconds,
            congested,
            server.as_ref().map_or(f64::NAN, |summary| summary.cpu_cores_mean),
            server.as_ref().map_or(f64::NAN, |summary| summary.cpu_cores_peak),
            server.as_ref().map_or(f64::NAN, |summary| summary.rss_mean_mb),
            server.as_ref().map_or(f64::NAN, |summary| summary.rss_peak_mb),
            client.as_ref().map_or(f64::NAN, |summary| summary.cpu_cores_mean),
            client.as_ref().map_or(f64::NAN, |summary| summary.rss_peak_mb),
        );
        if let Err(error) = fs::write(path, json) {
            eprintln!("bench: cannot write {path}: {error}");
            exit(1);
        }
        println!("json            {path}");
    }

    let healthy = joined == options.bots && lost == 0 && !rtt.is_empty();
    println!("BENCH: {}", if healthy { "PASS" } else { "DEGRADED" });
}
