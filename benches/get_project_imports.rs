#[macro_use]
extern crate criterion;

use criterion::Criterion;

use pprof::criterion::{Output, PProfProfiler};
use tach::imports::get_project_imports;

fn bench(c: &mut Criterion) {
    c.bench_function("get_project_imports", |b| {
        b.iter(|| get_project_imports(String::from("python"), String::from("tach/cli.py"), true))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = bench
}
criterion_main!(benches);
