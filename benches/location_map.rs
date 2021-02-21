use std::mem::MaybeUninit;

use criterion::*;
use legion::*;
use rand::{SeedableRng, prelude::{SliceRandom, StdRng}, thread_rng};
use world::{Allocate, LocationMap};

const ENTITY_COUNTS: [usize; 5] = [1_000, 10_000, 100_000, 1_000_000, 10_000_000];
const QUERY_COUNT: usize = 1_000_000;

fn bench_location_map(criterion: &mut Criterion) {
    for entity_count  in &ENTITY_COUNTS {
        let mut location_map = LocationMap::default();

        let allocate = Allocate::new();

        let entities = allocate.take(*entity_count).collect::<Vec<_>>();
        
        for entity in &entities {
            location_map.set(*entity, unsafe { MaybeUninit::uninit().assume_init() });
        }

        let mut rng = StdRng::from_seed([0; 32]);

        let choosen_entites = entities.choose_multiple(&mut rng, QUERY_COUNT)
            .cloned()    
            .collect::<Vec<_>>();
        
        criterion.bench_with_input(BenchmarkId::new("bench_location_map", entity_count), entity_count, |bencher, _| {
            bencher.iter(|| {
                for entity in &choosen_entites {
                    location_map.get(*entity);
                }
            });
        });       
    }
}

criterion_group!(
    basic,
    bench_location_map
);
criterion_main!(basic);