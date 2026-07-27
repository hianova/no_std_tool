use criterion::{black_box, criterion_group, criterion_main, Criterion};
use no_std_tool::bignum::U256;
use no_std_tool::linalg::Matrix;
use no_std_tool::math;
use no_std_tool::random::Xoshiro256StarStar;
use no_std_tool::sync::SpinMutex;

fn bench_math(c: &mut Criterion) {
    c.bench_function("math::silu_approx", |bencher| {
        bencher.iter(|| black_box(math::silu_approx_i8(black_box(10))))
    });

    c.bench_function("math::exp_q16", |bencher| {
        bencher.iter(|| black_box(math::exp_approx_q16(black_box(1 << 16))))
    });
}

fn bench_sync(c: &mut Criterion) {
    let lock = SpinMutex::new(0);
    c.bench_function("sync::spinlock_lock_unlock", |bencher| {
        bencher.iter(|| {
            let mut guard = lock.lock().unwrap();
            *guard = black_box(*guard + 1);
        })
    });
}

fn bench_random(c: &mut Criterion) {
    let mut rng = Xoshiro256StarStar::new(12345);
    c.bench_function("random::xoshiro_next", |bencher| {
        bencher.iter(|| black_box(rng.next_u64()))
    });
}

fn bench_bignum(c: &mut Criterion) {
    let x = U256([0x1234, 0x5678, 0x9ABC, 0xDEF0]);
    let y = U256([0x1, 0, 0, 0]);
    c.bench_function("bignum::u256_add", |bencher| {
        bencher.iter(|| black_box(black_box(x) + black_box(y)))
    });
}

fn bench_linalg(c: &mut Criterion) {
    let m1 = Matrix::<2, 2>::new([[1.0, 2.0], [3.0, 4.0]]);
    let m2 = Matrix::<2, 2>::new([[5.0, 6.0], [7.0, 8.0]]);
    c.bench_function("linalg::matrix_mul_2x2", |bencher| {
        bencher.iter(|| black_box(black_box(m1) * black_box(m2)))
    });
}

criterion_group!(benches, bench_math, bench_sync, bench_random, bench_bignum, bench_linalg);
criterion_main!(benches);
