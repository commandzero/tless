//! Opt-in native release comparison; excluded from ordinary CI timing assertions.
//! TLESS_REFERENCE and JLESS_REFERENCE name frozen release binaries. Optional
//! TLESS_CANDIDATE enables the acceptance gates. TLESS_PERFORMANCE_REPORT names
//! the JSON evidence file; TLESS_BUILD_PROVENANCE records reference build details.
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

struct Session(Child);
impl Drop for Session {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Fixture {
    let directory = std::env::temp_dir().join(format!("tless-performance-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let fixture = Fixture(directory);
    let mut file = io::BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(fixture.0.join("reference.json"))
            .unwrap(),
    );
    file.write_all(b"{\"records\":[").unwrap();
    for id in 0..150_000 {
        if id != 0 {
            file.write_all(b",").unwrap();
        }
        write!(
            file,
            "{{\"id\":{id},\"nested\":{{\"ok\":true,\"label\":\"record\"}},\"values\":[1,2,3]}}"
        )
        .unwrap();
    }
    file.write_all(b"]}").unwrap();
    file.flush().unwrap();
    assert_eq!(file.get_ref().metadata().unwrap().len(), 10_238_903);
    fixture
}
fn command_output(program: &str, args: &[&str]) -> String {
    let output = Command::new(program).args(args).output().unwrap();
    assert!(output.status.success(), "{program}: {:?}", output.stderr);
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
fn first_frame(binary: &Path, fixture: &Path, sample_memory: bool) -> (f64, Option<u64>) {
    let mut master = -1;
    let mut slave = -1;
    let mut size = libc::winsize {
        ws_row: 40,
        ws_col: 140,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // libc takes a mutable window-size pointer on macOS and a const pointer on Linux.
    let size_ptr = std::ptr::addr_of_mut!(size);
    // Isolated test PTY. Both descriptors become owned Files only after success.
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                size_ptr,
            )
        },
        0
    );
    let mut master = unsafe { File::from_raw_fd(master) };
    let slave = unsafe { File::from_raw_fd(slave) };
    let slave_fd = slave.as_raw_fd();
    let mut command = Command::new(binary);
    command
        .arg(fixture.join("reference.json"))
        .env("TERM", "xterm-256color")
        .env("XDG_CONFIG_HOME", fixture.join("config"))
        .env("HOME", fixture)
        .stdin(slave.try_clone().unwrap())
        .stdout(slave.try_clone().unwrap())
        .stderr(slave.try_clone().unwrap());
    // Only async-signal-safe terminal syscalls run between fork and exec.
    unsafe {
        command.pre_exec(move || {
            if libc::setsid() == -1 || libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let start = Instant::now();
    let mut child = Session(command.spawn().unwrap());
    drop(command);
    drop(slave);
    // The descriptor is owned above and remains open throughout this call.
    assert_ne!(
        unsafe { libc::fcntl(master.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
        -1
    );
    let ansi = regex::Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]").unwrap();
    let id = regex::Regex::new(r#"id"?:\s*0"#).unwrap();
    let mut output = Vec::new();
    let mut scanned = 0;
    let mut useful = None;
    let mut peak = None;
    let mut next_sample = Instant::now();
    loop {
        let mut buffer = [0; 65536];
        match master.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                output.extend_from_slice(&buffer[..n]);
                let requests = output[scanned..]
                    .windows(4)
                    .filter(|b| *b == b"\x1b[6n")
                    .count();
                scanned = output.len().saturating_sub(3);
                for _ in 0..requests {
                    master.write_all(b"\x1b[1;1R").unwrap();
                }
                if useful.is_none() {
                    let raw = String::from_utf8_lossy(&output);
                    let text = ansi.replace_all(&raw, "");
                    if raw.contains("\x1b[40;")
                        && id.is_match(&text)
                        && ["nested", "record", "reference.json"]
                            .iter()
                            .all(|s| text.contains(s))
                    {
                        useful = Some(start.elapsed().as_secs_f64() * 1000.0);
                        master.write_all(b"q").unwrap();
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
            Err(error) => panic!("{error}"),
        }
        if sample_memory && useful.is_none() && Instant::now() >= next_sample {
            let rss = command_output("ps", &["-o", "rss=", "-p", &child.0.id().to_string()]);
            let kib: u64 = rss.parse().unwrap();
            peak = Some(peak.unwrap_or(0).max(kib));
            next_sample = Instant::now() + Duration::from_millis(5);
        }
        if child.0.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "PTY timeout: {}",
            String::from_utf8_lossy(&output)
        );
        std::thread::sleep(Duration::from_micros(100));
    }
    let elapsed =
        useful.unwrap_or_else(|| panic!("No useful frame: {}", String::from_utf8_lossy(&output)));
    assert!(child.0.wait().unwrap().success());
    (elapsed, peak)
}

#[test]
#[ignore = "requires native frozen release binaries; run with --ignored --nocapture"]
fn compare_first_useful_frame() {
    let provenance = std::env::var("TLESS_BUILD_PROVENANCE")
        .expect("record compilers, revisions, features and build commands");
    let report = std::env::var("TLESS_PERFORMANCE_REPORT").expect("JSON evidence output path");
    let mut binaries = vec![
        (
            "jless",
            PathBuf::from(std::env::var_os("JLESS_REFERENCE").expect("jless 0.9.0 release binary")),
        ),
        (
            "baseline",
            PathBuf::from(std::env::var_os("TLESS_REFERENCE").expect("9bb9568 release binary")),
        ),
    ];
    if let Some(candidate) = std::env::var_os("TLESS_CANDIDATE") {
        binaries.push(("candidate", PathBuf::from(candidate)));
    }
    let fixture = fixture();
    let mut warmups = Vec::new();
    let mut timings = vec![Vec::new(); binaries.len()];
    for (_, binary) in &binaries {
        warmups.push(first_frame(binary, &fixture.0, false).0);
    }
    for round in 0..20 {
        for offset in 0..binaries.len() {
            let index = (round + offset) % binaries.len();
            timings[index].push(first_frame(&binaries[index].1, &fixture.0, false).0);
        }
    }
    let mut results = Vec::new();
    let mut summaries = Vec::new();
    for (i, (name, binary)) in binaries.iter().enumerate() {
        let mut sorted = timings[i].clone();
        sorted.sort_by(f64::total_cmp);
        let median = (sorted[9] + sorted[10]) / 2.0;
        let p95 = sorted[18];
        let (_, peak) = first_frame(binary, &fixture.0, true);
        assert!(peak.is_some(), "no pre-frame RSS sample");
        summaries.push((median, p95));
        results.push(serde_json::json!({"name": name, "binary": binary,
            "sha256": command_output("shasum", &["-a", "256", binary.to_str().unwrap()]),
            "version": command_output(binary.to_str().unwrap(), &["--version"]),
            "warmup_ms": warmups[i], "runs_ms": timings[i], "median_ms": median,
            "p95_ms": p95, "sampled_startup_peak_kib": peak}));
        println!("{name}: median={median:.2}ms p95={p95:.2}ms peak={peak:?}KiB");
    }
    let evidence = serde_json::json!({"host": command_output("uname", &["-a"]),
        "build_provenance": provenance, "fixture_bytes": 10_238_903, "columns": 140, "rows": 40,
        "frame": "initial id: 0, nested, record, filename, and final-row cursor; not alternate-screen entry",
        "timing": "before spawn through useful frame; one warmup then 20 runs; nearest-rank p95",
        "memory": "separate untimed launch; ps RSS in KiB sampled at least 5ms apart until frame detection; sampled peak, not exit high-water mark",
        "results": results});
    std::fs::write(report, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    if binaries.len() == 3 {
        assert!(
            summaries[2].0 <= summaries[0].0 * 3.0,
            "median exceeds 3x jless"
        );
        assert!(
            summaries[2].1 <= summaries[0].1 * 3.0,
            "p95 exceeds 3x jless"
        );
        assert!(
            summaries[2].0 <= summaries[1].0 / 5.0,
            "median lacks 5x baseline improvement"
        );
    }
}
