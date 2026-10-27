use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde_json::{Value as J, json};

use super::imp::run;

/// A loopback peer that accepts one connection and never reads from it; returns its
/// `host:port` and the accepted stream's keeper (drop it to let the connection go).
fn stalled_peer() -> (String, mpsc::Sender<()>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let host = l.local_addr().unwrap().to_string();
    let (stop, wait) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        let (_s, _): (TcpStream, _) = l.accept().unwrap();
        let _ = wait.recv_timeout(Duration::from_secs(120));
    });
    (host, stop)
}

fn open(host: &str) -> u32 {
    let r = run("open", 0, &json!({"host": host, "timeout": 5}));
    r["id"].as_u64().unwrap_or_else(|| panic!("open {host}: {r}")) as u32
}

/// A bounded chunk large enough to fill a stalled peer's buffers. Windows can accept the
/// first entire send request even when the peer isn't reading, so prime it first.
fn big() -> String {
    "x".repeat(2 * 1024 * 1024)
}

/// Establish and verify backpressure before measuring a subsequent write. The amount a
/// loopback stack queues differs by platform, so send bounded chunks until one times out.
fn prime(id: u32) {
    let p = json!({"data": big(), "timeout": 0.1});
    for _ in 0..32 {
        let r = run("write", id, &p);
        if r["error"].as_str().is_some_and(|e| e.contains("timed out")) {
            return;
        }
        assert_eq!(r["ok"], true, "{r}");
    }
    panic!("the stalled peer did not exert backpressure after 64 MiB");
}

/// Run `f` on a thread; its result, or `None` if it takes longer than `limit`.
fn within<T: Send + 'static>(limit: Duration, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(limit).ok()
}

#[test]
fn a_write_to_a_peer_that_stops_reading_fails_after_the_timeout() {
    // #1248: it used to block for good.
    let (host, _keep) = stalled_peer();
    let id = open(&host);
    prime(id);
    let data = big();
    let t0 = Instant::now();
    let r = within(Duration::from_secs(20), move || run("write", id, &json!({"data": data, "timeout": 1}))).expect("the write gave up");
    assert!(r["error"].as_str().is_some_and(|e| e.contains("timed out")), "{r}");
    assert!(t0.elapsed() < Duration::from_secs(15), "{:?}", t0.elapsed());
    run("close", id, &J::Null);
}

#[test]
fn a_blocked_socket_doesnt_hold_up_the_others_and_close_interrupts_it() {
    let (host_a, _keep_a) = stalled_peer();
    let (host_b, _keep_b) = stalled_peer();
    let (a, b) = (open(&host_a), open(&host_b));
    prime(a);
    // A write on A that would wait a minute.
    let data = big();
    let (tx, blocked) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(run("write", a, &json!({"data": data, "timeout": 60})));
    });
    std::thread::sleep(Duration::from_millis(500));
    // Socket B answers meanwhile (the table isn't locked by A's write).
    let r = within(Duration::from_secs(3), move || run("connected", b, &json!({}))).expect("socket B answered while A was blocked");
    assert_eq!(r["value"], true, "{r}");
    // Closing A returns at once and ends A's write.
    within(Duration::from_secs(3), move || run("close", a, &J::Null)).expect("close returned while A was blocked");
    let r = blocked.recv_timeout(Duration::from_secs(5)).expect("A's write ended when A was closed");
    assert!(r.get("error").is_some(), "{r}");
    run("close", b, &J::Null);
}
