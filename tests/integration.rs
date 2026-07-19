extern crate std;
use no_std_tool::math::{FIXED_POINT_ONE, exp_approx_q16, rsqrt_approx_i32, silu_approx_i8};

fn run_exp_approx_common() {
    core::hint::black_box(exp_approx_q16(0 * FIXED_POINT_ONE));
    core::hint::black_box(exp_approx_q16(-FIXED_POINT_ONE));
    core::hint::black_box(exp_approx_q16(-2 * FIXED_POINT_ONE));
    core::hint::black_box(exp_approx_q16(FIXED_POINT_ONE));
    core::hint::black_box(exp_approx_q16(2 * FIXED_POINT_ONE));
    core::hint::black_box(exp_approx_q16(-11 * FIXED_POINT_ONE));
    core::hint::black_box(exp_approx_q16(11 * FIXED_POINT_ONE));
    
    if let Some(seed) = std::env::var("COVOPT_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
    {
        core::hint::black_box(exp_approx_q16((seed % 15) * FIXED_POINT_ONE));
        core::hint::black_box(exp_approx_q16(-(seed % 15) * FIXED_POINT_ONE));
    }
}

#[test]
fn test_exp_approx() {
    let mut handles = std::vec::Vec::new(); let (tx, rx) = std::sync::mpsc::channel();
    for _ in 0..4 {
        let tx_clone = tx.clone(); let handle = std::thread::spawn(move || {
            run_exp_approx_common();
            std::hint::black_box(());
            tx_clone.send(()).unwrap(); 
        });
        handles.push(handle);
    }
    for _ in 0..4 {
        rx.recv_timeout(std::time::Duration::from_secs(5)).expect("Watchdog timeout");
    }
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_silu_approx() {
    let mut handles = std::vec::Vec::new(); let (tx, rx) = std::sync::mpsc::channel();
    for _ in 0..4 {
        let tx_clone = tx.clone(); let handle = std::thread::spawn(move || {
            let _ = silu_approx_i8(10);
            let _ = silu_approx_i8(11);
            let _ = silu_approx_i8(12);
            std::hint::black_box(());
            tx_clone.send(()).unwrap(); 
        });
        handles.push(handle);
    }
    for _ in 0..4 {
        rx.recv_timeout(std::time::Duration::from_secs(5)).expect("Watchdog timeout");
    }
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_exp_approx_edge_cases() {
    run_exp_approx_common();
}

#[test]
fn test_rsqrt_approx_edge_cases() {
    assert_eq!(rsqrt_approx_i32(0), None);
    assert!(rsqrt_approx_i32(1).unwrap() > 0);
}
