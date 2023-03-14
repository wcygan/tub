mod src;

extern crate criterion;
extern crate tub;

use criterion::{criterion_main, Criterion};
use std::time::Duration;

fn default_config() -> Criterion {
    Criterion::default()
        // Make sure you've installed gnuplot (e.g., `brew install gnuplot`)
        .plotting_backend(criterion::PlottingBackend::Gnuplot)
        .with_plots()
        .nresamples(5000)
        .sample_size(100)
        .warm_up_time(Duration::from_secs(4))
        .measurement_time(Duration::from_secs(5))
}

criterion_main!(src::fixed_group, src::scaled_group);
