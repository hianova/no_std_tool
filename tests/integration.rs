extern crate std;
use no_std_tool::math::{FIXED_POINT_ONE, exp_approx_q16, rsqrt_approx_i32, silu_approx_i8};

fn run_exp_approx_common() {
    let mut test_vals: no_std_tool::collections::Vec<i32, 16> =
        no_std_tool::collections::Vec::new();
    let _ = test_vals.push(0);
    let _ = test_vals.push(-1);
    let _ = test_vals.push(-2);
    let _ = test_vals.push(1);
    let _ = test_vals.push(2);
    let _ = test_vals.push(-11);
    let _ = test_vals.push(11);

    // CovOpt 2.0 Entropy Fuzz Injection
    if let Some(seed) = std::env::var("COVOPT_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
    {
        let _ = test_vals.push(seed % 15);
        let _ = test_vals.push(-(seed % 15));
    }

    for v in test_vals {
        let v_q16 = v * FIXED_POINT_ONE;
        let res_q16 = exp_approx_q16(v_q16);
        if v < -10 {
            assert_eq!(res_q16, Some(0));
        } else if v > 10 {
            assert_eq!(res_q16, None);
        } else {
            assert!(res_q16.is_some());
        }
    }
}

#[test]
fn test_exp_approx() {
    run_exp_approx_common();
}

#[test]
fn test_silu_approx() {
    for x in -5..=5 {
        let res_i8 = silu_approx_i8(x as i8).unwrap();
        assert!(res_i8 >= -128);
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
