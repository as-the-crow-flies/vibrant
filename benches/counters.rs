// GPU performance counters (occupancy, limiters, bandwidth, …) per shader, from
// Instruments: runs the selected harness passes in a loop while `xctrace`
// records this process, then averages every counter over the time each Metal
// encoder (by label) was running. Labels are paths, one pass per shader
// ("line/occupancy/voxelize"): every prefix that branches is a group, with its
// shaders' time summed and counters averaged over it. Each run is saved under
// target/counters/ and the next run prints its change against it.
//
//   cargo bench --bench counters -- volume/trace           # shaders whose label contains it
//   cargo bench --bench counters -- line/occupancy/voxelize  # one shader of a pass
//   cargo bench --bench counters -- cascade/0 --all        # every counter
//   cargo bench --bench counters -- --seconds 3 line/trace # default 1.2; ~1 s of it is attaching
//   TRACED=1 cargo bench --bench counters -- volume/trace
//
// Needs Xcode and an Instruments template with the "GPU" instrument and "Metal
// GPU Counters" with a counter set (--template, default MetalPerformanceLimiters);
// Metal itself only exposes timestamps. Counters are system-wide: keep other
// GPU work (and recordings) off the machine meanwhile. The GPU driver samples
// every counter every 50 µs (templates can't change it) and Instruments exports
// them as XML at ~30k rows/s, so a default run takes about two minutes.

mod harness;

#[cfg(target_vendor = "apple")]
fn main() {
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::Duration;

    let options = Options::parse();

    let mut bench = harness::setup();
    bench.controller.settings_mut().ray_traced = std::env::var("TRACED").is_ok();
    bench.controller.settings_mut().mirror_fresnel = std::env::var("MIRROR").is_ok();
    bench.prime_frame();

    // A filter selects the passes whose name contains it, or else the pass it
    // names a shader of ("line/occupancy/voxelize" runs line/occupancy); the
    // output then keeps the shaders containing it.
    let b = &bench;
    let all: Vec<_> = b.passes().into_iter().chain(b.cascade_levels()).collect();
    let selected = |name: &str| {
        options.filters.iter().any(|f| {
            name.contains(f.as_str())
                || (all.iter().all(|pass| !pass.name.contains(f.as_str()))
                    && f.starts_with(&format!("{name}/")))
        })
    };
    let mut passes: Vec<_> = all
        .iter()
        .filter(|pass| options.filters.is_empty() || selected(pass.name))
        .collect();
    // Don't run a part of another selected pass twice (cascade/0 of cascade).
    let names: Vec<&str> = passes.iter().map(|pass| pass.name).collect();
    passes.retain(|pass| {
        !names
            .iter()
            .any(|n| pass.name.starts_with(&format!("{n}/")))
    });
    if passes.is_empty() {
        eprintln!(
            "no pass matches {:?}; passes: {}",
            options.filters,
            all.iter()
                .map(|pass| pass.name)
                .collect::<Vec<_>>()
                .join(", ")
        );
        return;
    }
    eprintln!(
        "passes: {}",
        passes
            .iter()
            .map(|pass| pass.name)
            .collect::<Vec<_>>()
            .join(", ")
    );

    let run = |pass: &&harness::Pass| b.time_pass(|cmd| (pass.record)(cmd));
    for _ in 0..3 {
        passes.iter().for_each(run);
    }

    let work = options
        .keep
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join(format!("counters-{}", std::process::id())));
    std::fs::create_dir_all(&work).unwrap();
    let trace = work.join("counters.trace");
    let _ = std::fs::remove_dir_all(&trace);

    // Record this process; run the passes from when the recording starts until
    // xctrace stops it.
    let pid = std::process::id();
    let mut recorder = Command::new("xcrun")
        .args([
            "xctrace",
            "record",
            "--template",
            &options.template,
            "--attach",
            &pid.to_string(),
        ])
        .args(["--time-limit", &format!("{}s", options.seconds), "--output"])
        .arg(&trace)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("xcrun xctrace (needs Xcode)");

    let (started, recording) = mpsc::channel();
    let stdout = recorder.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line.contains("Ctrl-C") {
                let _ = started.send(());
            }
        }
    });
    recording
        .recv_timeout(Duration::from_secs(120))
        .expect("xctrace didn't start recording");

    let mut rounds = 0;
    while recorder.try_wait().unwrap().is_none() {
        passes.iter().for_each(run);
        rounds += 1;
    }
    assert!(recorder.wait().unwrap().success(), "xctrace failed");
    eprintln!("recorded {rounds} rounds; exporting (a few minutes)...");

    let intervals = export(&trace, "metal-gpu-intervals", &work);
    let samples = export(&trace, "metal-gpu-counter-intervals", &work);

    // Instruments names an unlabelled encoder after its command buffer:
    // "Unlabelled" (Gpu::cmd) or wgpu's "(wgpu internal) …".
    let unlabelled = |label: &str| label == "Unlabelled" || label.starts_with("(wgpu internal)");

    // This process's encoders: (start, end, label), by start. Instruments
    // splits a render pass into vertex and fragment intervals with their own
    // encoder ids: merge them by command buffer (column 15) and label.
    let own = format!(" ({pid}))");
    let mut encoders: HashMap<(String, String), (u64, u64)> = HashMap::new();
    for_each_row(&intervals, |row| {
        let label = row
            .get(6)
            .and_then(|cell| cell.0.clone())
            .unwrap_or_default();
        if label.contains(&own) {
            let start: u64 = row[0].1.trim().parse().unwrap();
            let end = start + row[1].1.trim().parse::<u64>().unwrap();
            let name = label.split(':').next().unwrap().trim().to_string();
            let span = encoders
                .entry((row[15].1.clone(), name))
                .or_insert((start, end));
            *span = (span.0.min(start), span.1.max(end));
        }
    });
    let mut spans: Vec<(u64, u64, String)> = encoders
        .into_iter()
        .map(|((_, name), (start, end))| (start, end, name))
        .collect();
    spans.retain(|(_, _, label)| {
        options.filters.is_empty()
            || unlabelled(label)
            || options.filters.iter().any(|f| label.contains(f.as_str()))
    });
    spans.sort();
    assert!(
        !spans.is_empty(),
        "no GPU work from this process in the recording: GPU tracing only starts ~1 s into it, try a longer --seconds"
    );

    // Every label prefix that splits into two or more branches is a group of
    // the shaders below it; `keys` is what each shader's time counts towards.
    let leaves: BTreeSet<String> = spans.iter().map(|span| span.2.clone()).collect();
    let mut branches: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for leaf in &leaves {
        let parts: Vec<&str> = leaf.split('/').collect();
        for i in 1..parts.len() {
            branches
                .entry(parts[..i].join("/"))
                .or_default()
                .insert(parts[i].to_string());
        }
    }
    let groups: Vec<String> = branches
        .into_iter()
        .filter(|(_, branch)| branch.len() > 1)
        .map(|(group, _)| group)
        .collect();
    let within = |label: &str, group: &str| label.starts_with(&format!("{group}/"));
    let keys: HashMap<String, Vec<String>> = leaves
        .iter()
        .map(|leaf| {
            let groups = groups.iter().filter(|group| within(leaf, group)).cloned();
            (
                leaf.clone(),
                std::iter::once(leaf.clone()).chain(groups).collect(),
            )
        })
        .collect();

    // Every counter averaged over the time of each label's encoders, and of
    // each group's.
    let starts: Vec<u64> = spans.iter().map(|span| span.0).collect();
    let longest = spans.iter().map(|span| span.1 - span.0).max().unwrap();
    let mut sums: HashMap<(String, String), (f64, f64)> = HashMap::new();
    for_each_row(&samples, |row| {
        let start: u64 = row[0].1.trim().parse().unwrap();
        let end = start + row[1].1.trim().parse::<u64>().unwrap();
        let name = row[3].0.clone().unwrap_or_default();
        let Ok(value) = row[5].1.trim().parse::<f64>() else {
            return;
        };
        let first = starts.partition_point(|&s| s < start.saturating_sub(longest));
        let last = starts.partition_point(|&s| s <= end);
        for (span_start, span_end, label) in &spans[first..last] {
            let overlap = end.min(*span_end) as f64 - start.max(*span_start) as f64;
            if overlap > 0.0 {
                for key in &keys[label] {
                    let sum = sums.entry((key.clone(), name.clone())).or_default();
                    sum.0 += value * overlap;
                    sum.1 += overlap;
                }
            }
        }
    });

    // Per shader: total ms and encoder count.
    let mut timings: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for (start, end, label) in &spans {
        let entry = timings.entry(label.clone()).or_default();
        entry.0 += (end - start) as f64 / 1e6;
        entry.1 += 1;
    }

    let mut values: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    for ((label, name), (sum, weight)) in sums {
        values.entry(label).or_default().insert(name, sum / weight);
    }
    // A group's time is its shaders' summed.
    for (label, (total, count)) in &timings {
        for key in &keys[label] {
            *values
                .entry(key.clone())
                .or_default()
                .entry("GPU ms per encoder".into())
                .or_default() += total / *count as f64;
        }
    }

    // Depth-first, each level by time: a group, then what's below it.
    let mut nodes: Vec<(String, Option<String>)> = values
        .keys()
        .map(|label| {
            let parent = groups
                .iter()
                .filter(|group| within(label, group))
                .max_by_key(|group| group.len())
                .cloned();
            (label.clone(), parent)
        })
        .collect();
    let ms = |label: &String| values[label]["GPU ms per encoder"];
    nodes.sort_by(|a, b| ms(&b.0).total_cmp(&ms(&a.0)));
    fn visit(
        nodes: &[(String, Option<String>)],
        parent: Option<&String>,
        depth: usize,
        out: &mut Vec<(String, usize)>,
    ) {
        for (label, _) in nodes.iter().filter(|node| node.1.as_ref() == parent) {
            out.push((label.clone(), depth));
            visit(nodes, Some(label), depth + 1, out);
        }
    }
    let mut order = Vec::new();
    visit(&nodes, None, 0, &mut order);

    // Counter sections to print; with --all, the counters in none of them go
    // last.
    let mut sections: Vec<(String, Summary, Vec<String>)> = SECTIONS
        .iter()
        .map(|(name, summary, counters)| {
            let counters = counters.iter().map(|c| c.to_string()).collect();
            (name.to_string(), *summary, counters)
        })
        .collect();
    if options.all {
        let listed: Vec<String> = sections
            .iter()
            .flat_map(|section| section.2.clone())
            .chain(["GPU ms per encoder".to_string()])
            .collect();
        let mut other: Vec<String> = values.values().flat_map(|v| v.keys().cloned()).collect();
        other.sort();
        other.dedup();
        other.retain(|name| !listed.contains(name));
        sections.push(("Other".into(), Summary::None, other));
    }

    // Each section's summary is stored like a counter, so it gets deltas and
    // baselines too.
    for (section, summary, counters) in &mut sections {
        let fold = match summary {
            Summary::None => continue,
            Summary::Max => f64::max,
            Summary::Sum => |a: f64, b: f64| a + b,
        };
        *section = format!("{section} ({})", summary.name());
        for counters_of in values.values_mut() {
            let present: Vec<f64> = counters
                .iter()
                .filter_map(|c| counters_of.get(c))
                .copied()
                .collect();
            if let Some(total) = present.into_iter().reduce(fold) {
                counters_of.insert(section.clone(), total);
            }
        }
    }

    let baselines = baseline_dir();
    let previous: HashMap<String, HashMap<String, f64>> = order
        .iter()
        .map(|(label, _)| (label.clone(), load_baseline(&baselines, label)))
        .collect();

    let style = Style::detect();
    let width = sections
        .iter()
        .flat_map(|(section, _, counters)| {
            std::iter::once(section.len()).chain(counters.iter().map(|c| c.len() + 2))
        })
        .max()
        .unwrap_or(0);
    for (label, depth) in order
        .iter()
        .filter(|(label, _)| !unlabelled(label) || values[label]["GPU ms per encoder"] >= 0.05)
    {
        if unlabelled(label) {
            println!(
                "\n{}",
                style.dim("(unlabelled pass: give its begin_*_pass a label to name it)")
            );
        }
        let (values, previous) = (&values[label], &previous[label]);
        let ms = values["GPU ms per encoder"];
        let detail = match timings.get(label) {
            Some((_, count)) => format!("×{count} encoders"),
            None => format!(
                "group of {}",
                leaves.iter().filter(|leaf| within(leaf, label)).count()
            ),
        };
        let title = format!("{}{label}  {ms:.2} ms", "│ ".repeat(*depth));
        println!(
            "\n{}  {}{}",
            style.bold(&title),
            style.delta(ms, previous.get("GPU ms per encoder"), true),
            style.dim(&format!("  {detail}")),
        );
        for (section, summary, counters) in &sections {
            if *summary == Summary::None && counters.iter().all(|c| !values.contains_key(c)) {
                continue;
            }
            let heat = *summary == Summary::Max;
            let line = |title: &str, name: &str, bold: bool| {
                let Some(&value) = values.get(name) else {
                    return;
                };
                let unit = unit(name);
                let text = format!("{title:<width$}  {value:>8.2} {unit:<4}");
                let text = if bold { style.bold(&text) } else { text };
                let text = if value.abs() < 1.0 {
                    style.dim(&text)
                } else if heat && unit == "%" {
                    style.heat(&text, value)
                } else {
                    text
                };
                let bar = if unit == "%" {
                    bar(value)
                } else {
                    " ".repeat(BAR)
                };
                println!(
                    "  {text}  {}  {}",
                    style.dim(&bar),
                    style.delta(value, previous.get(name), false)
                );
            };
            match summary {
                Summary::None => println!("  {}", style.bold(section)),
                _ => line(section, section, true),
            }
            for name in counters {
                line(&format!("  {name}"), name, false);
            }
        }
    }

    for (label, _) in &order {
        save_baseline(&baselines, label, &values[label]);
    }
    if options.keep.is_none() {
        let _ = std::fs::remove_dir_all(&work);
    }
}

#[cfg(not(target_vendor = "apple"))]
fn main() {
    eprintln!("counters: Metal (Instruments) only");
}

/// How a section sums up its counters: limiters by the busiest (a mean would be
/// diluted by idle units, e.g. F16 or fragment launch in a compute kernel),
/// bandwidths by their total.
#[cfg(target_vendor = "apple")]
#[derive(Clone, Copy, PartialEq)]
enum Summary {
    None,
    Max,
    Sum,
}

#[cfg(target_vendor = "apple")]
impl Summary {
    fn name(self) -> &'static str {
        match self {
            Summary::None => "",
            Summary::Max => "max",
            Summary::Sum => "total",
        }
    }
}

#[cfg(target_vendor = "apple")]
const SECTIONS: &[(&str, Summary, &[&str])] = &[
    (
        "Occupancy",
        Summary::None,
        &[
            "Kernel Occupancy",
            "FS Occupancy",
            "Occupancy Manager Target",
        ],
    ),
    (
        "Arithmetic",
        Summary::Max,
        &[
            "ALU Utilization",
            "Instruction Throughput Limiter",
            "F32 Limiter",
            "F16 Limiter",
            "Integer and Complex Limiter",
            "Integer and Conditional Limiter",
            "Control Flow Limiter",
        ],
    ),
    (
        "Memory",
        Summary::Max,
        &[
            "L1 Cache Limiter",
            "Last Level Cache Limiter",
            "Texture Read Limiter",
            "Texture Read Cache Limiter",
            "MMU Limiter",
        ],
    ),
    (
        "Launch",
        Summary::Max,
        &[
            "Compute Shader Launch Limiter",
            "Fragment Shader Launch Limiter",
        ],
    ),
    (
        "L1 bandwidth",
        Summary::Sum,
        &[
            "Register L1 Read Bandwidth",
            "Stack L1 Read Bandwidth",
            "Threadgroup Memory L1 Read Bandwidth",
        ],
    ),
    (
        "DRAM bandwidth",
        Summary::Sum,
        &["GPU Read Bandwidth", "GPU Write Bandwidth"],
    ),
];

#[cfg(target_vendor = "apple")]
struct Options {
    filters: Vec<String>,
    seconds: f32,
    template: String,
    all: bool,
    keep: Option<std::path::PathBuf>,
}

#[cfg(target_vendor = "apple")]
impl Options {
    fn parse() -> Self {
        let mut options = Options {
            filters: Vec::new(),
            seconds: 1.2,
            template: "MetalPerformanceLimiters".into(),
            all: false,
            keep: None,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--seconds" => {
                    options.seconds = args
                        .next()
                        .and_then(|v| v.parse().ok())
                        .expect("--seconds <s>")
                }
                "--template" => options.template = args.next().expect("--template <name>"),
                "--keep" => options.keep = args.next().map(Into::into),
                "--all" => options.all = true,
                // cargo bench passes --bench to every harness = false target.
                "--bench" => {}
                _ => options.filters.push(arg),
            }
        }
        options
    }
}

/// Exports one table of the trace to `<work>/<schema>.xml`.
#[cfg(target_vendor = "apple")]
fn export(trace: &std::path::Path, schema: &str, work: &std::path::Path) -> std::path::PathBuf {
    let path = work.join(format!("{schema}.xml"));
    let file = std::fs::File::create(&path).unwrap();
    let status = std::process::Command::new("xcrun")
        .args(["xctrace", "export", "--input"])
        .arg(trace)
        .args([
            "--xpath",
            &format!(r#"/trace-toc/run[@number="1"]/data/table[@schema="{schema}"]"#),
        ])
        .stdout(file)
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(
        status.success(),
        "xctrace export {schema} failed: the template needs the instrument that records it \
         (metal-gpu-intervals: \"GPU\"; metal-gpu-counter-intervals: \"Metal GPU Counters\")"
    );
    path
}

/// Calls `visit` with each row's top-level cells as (fmt attribute, text),
/// resolving Instruments' `ref` cells to the `id`'d cell they repeat.
#[cfg(target_vendor = "apple")]
fn for_each_row(path: &std::path::Path, mut visit: impl FnMut(&[(Option<String>, String)])) {
    use quick_xml::events::{BytesStart, Event};
    use std::collections::HashMap;

    struct Open {
        id: Option<String>,
        fmt: Option<String>,
        text: String,
        top: bool,
    }

    let attribute = |element: &BytesStart, name: &str| -> Option<String> {
        element
            .try_get_attribute(name)
            .ok()
            .flatten()
            .map(|attr| String::from_utf8_lossy(&attr.value).into_owned())
    };

    let mut reader = quick_xml::Reader::from_file(path).unwrap();
    let mut buf = Vec::new();
    let mut known: HashMap<String, (Option<String>, String)> = HashMap::new();
    let mut row: Vec<(Option<String>, String)> = Vec::new();
    let mut open: Vec<Open> = Vec::new();
    let mut in_row = false;

    loop {
        match reader.read_event_into(&mut buf).unwrap() {
            Event::Start(element) => {
                if element.name().as_ref() == b"row" {
                    in_row = true;
                    row.clear();
                } else if in_row {
                    open.push(Open {
                        id: attribute(&element, "id"),
                        fmt: attribute(&element, "fmt"),
                        text: String::new(),
                        top: open.is_empty(),
                    });
                }
            }
            Event::Text(text) => {
                if let Some(top) = open.last_mut() {
                    top.text.push_str(&String::from_utf8_lossy(&text));
                }
            }
            Event::End(element) => {
                if element.name().as_ref() == b"row" {
                    in_row = false;
                    visit(&row);
                } else if let Some(element) = open.pop() {
                    let cell = (element.fmt, element.text);
                    if let Some(id) = element.id {
                        known.insert(id, cell.clone());
                    }
                    if element.top {
                        row.push(cell);
                    }
                }
            }
            Event::Empty(element) => {
                if in_row && open.is_empty() {
                    let cell = match attribute(&element, "ref") {
                        Some(reference) => known.get(&reference).cloned().unwrap_or_default(),
                        None => (attribute(&element, "fmt"), String::new()),
                    };
                    row.push(cell);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
}

#[cfg(target_vendor = "apple")]
fn baseline_dir() -> std::path::PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/target").into());
    let dir = std::path::Path::new(&target).join("counters");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(target_vendor = "apple")]
fn baseline_file(dir: &std::path::Path, label: &str) -> std::path::PathBuf {
    let name: String = label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    dir.join(format!("{name}.tsv"))
}

#[cfg(target_vendor = "apple")]
fn load_baseline(dir: &std::path::Path, label: &str) -> std::collections::HashMap<String, f64> {
    std::fs::read_to_string(baseline_file(dir, label))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once('\t')?;
            Some((name.to_string(), value.parse().ok()?))
        })
        .collect()
}

#[cfg(target_vendor = "apple")]
fn save_baseline(
    dir: &std::path::Path,
    label: &str,
    values: &std::collections::BTreeMap<String, f64>,
) {
    let text: String = values
        .iter()
        .map(|(name, value)| format!("{name}\t{value}\n"))
        .collect();
    std::fs::write(baseline_file(dir, label), text).unwrap();
}

#[cfg(target_vendor = "apple")]
const BAR: usize = 20;

#[cfg(target_vendor = "apple")]
fn bar(percent: f64) -> String {
    let full = ((percent / 100.0).clamp(0.0, 1.0) * BAR as f64).round() as usize;
    "█".repeat(full) + &"·".repeat(BAR - full)
}

#[cfg(target_vendor = "apple")]
fn unit(name: &str) -> &'static str {
    if name.ends_with(" ms per encoder") {
        "ms"
    } else if name.to_lowercase().contains("bandwidth") {
        "GB/s"
    } else if ["Limiter", "Utilization", "Occupancy", "Target", "(max)"]
        .iter()
        .any(|word| name.contains(word))
    {
        "%"
    } else {
        ""
    }
}

/// ANSI styling, off when stdout isn't a terminal or NO_COLOR is set.
#[cfg(target_vendor = "apple")]
struct Style(bool);

#[cfg(target_vendor = "apple")]
impl Style {
    fn detect() -> Self {
        use std::io::IsTerminal;
        Style(std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none())
    }

    fn paint(&self, text: &str, code: &str) -> String {
        if self.0 {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn bold(&self, text: &str) -> String {
        self.paint(text, "1")
    }

    fn dim(&self, text: &str) -> String {
        self.paint(text, "2")
    }

    /// Red when a limiter is near saturation, yellow when it's busy.
    fn heat(&self, text: &str, percent: f64) -> String {
        match percent {
            p if p >= 80.0 => self.paint(text, "31"),
            p if p >= 50.0 => self.paint(text, "33"),
            _ => text.to_string(),
        }
    }

    /// Change against the baseline; within noise (±3%) or from a near-zero
    /// baseline it's dimmed. Only time is judged: green faster, red slower.
    fn delta(&self, value: f64, before: Option<&f64>, time: bool) -> String {
        let Some(&before) = before else {
            return String::new();
        };
        if before.abs() < 0.1 {
            return String::new();
        }
        let change = 100.0 * (value - before) / before.abs();
        let text = format!("{change:+.1}%");
        match change {
            c if c.abs() < 3.0 => self.dim(&text),
            c if time && c < 0.0 => self.paint(&text, "32"),
            _ if time => self.paint(&text, "31"),
            _ => text,
        }
    }
}
