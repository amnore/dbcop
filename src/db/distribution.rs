extern crate rand;

use rand::prelude::{Distribution, ThreadRng};

pub struct MyDistribution<D: Distribution<usize>> {
    d: D,
}

pub trait MyDistributionTrait {
    fn sample(&self, rng: &mut ThreadRng) -> usize;
}

impl<D: Distribution<usize>> MyDistributionTrait for MyDistribution<D> {
    fn sample(&self, rng: &mut ThreadRng) -> usize {
        self.d.sample(rng)
    }
}

impl<D: Distribution<usize>> MyDistribution<D> {
    pub fn new(d: D) -> MyDistribution<D> {
        MyDistribution { d }
    }
}
