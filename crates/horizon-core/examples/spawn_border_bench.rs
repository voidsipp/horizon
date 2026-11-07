//! Microbenchmark the actual border/ticket predicates, separately from world generation.
//! cargo run --release -p horizon-core --example spawn_border_bench -- 4000
use horizon_core::{fast_hash, geometry::border::BorderGeometry};
use std::{hint::black_box, sync::Arc, time::Instant};
#[allow(dead_code)]
#[path = "../../horizon-client/src/ticket_levels.rs"]
mod ticket_levels;
use ticket_levels::{ChunkTickets, Levels, Sources};

type Chunk = (i32, i32);
type Residents = fast_hash::FxHashMap<Chunk, Arc<()>>;

#[inline(never)]
fn corners(border: &BorderGeometry, (x, z): Chunk) -> bool {
    border.is_within_bounds(f64::from(x << 4), f64::from(z << 4), 0.0)
        && border.is_within_bounds(
            f64::from((x << 4).wrapping_add(15)),
            f64::from((z << 4).wrapping_add(15)),
            0.0,
        )
}
#[inline(never)]
fn extent(border: &BorderGeometry, (x, z): Chunk) -> bool {
    border.is_chunk_within_bounds(x, z)
}

fn measure(mut pass: impl FnMut() -> usize, calls: usize, rounds: usize, expected: usize) -> f64 {
    assert_eq!(pass(), expected);
    for _ in 0..64 {
        black_box(pass());
    }
    let start = Instant::now();
    for _ in 0..rounds {
        black_box(pass());
    }
    start.elapsed().as_nanos() as f64 / (calls * rounds) as f64
}
fn main() {
    let rounds = std::env::args().nth(1).map_or(4000, |s| s.parse().unwrap());
    let chunks: Vec<Chunk> = (-8..=8)
        .flat_map(|x| (-8..=8).map(move |z| (x, z)))
        .collect();
    // ChunkMap dereferences to this same FxHashMap with a one-word Arc value.
    let residents: Residents = chunks.iter().map(|&c| (c, Arc::new(()))).collect();
    let mut tickets = ChunkTickets::default();
    tickets.update(Some(Sources {
        players: vec![(0, 0)],
        forced: Vec::new(),
        portal: Vec::new(),
        dragon: None,
        view_distance: 8,
        simulation_distance: 4,
    }));
    let levels: Levels = tickets.levels();
    println!("scenario,moving,variant,median_ns_per_candidate,min_ns,max_ns,accepted");
    for moving in [false, true] {
        for (scenario, center, size) in [
            ("inside", [0.0, 0.0], 4096.0),
            ("edge", [8.0, 8.0], 128.0),
            ("outside", [4096.0, 4096.0], 128.0),
        ] {
            let mut border = BorderGeometry::new(center, size, 29_999_984).unwrap();
            if moving {
                border.lerp_size_between(size, size * 0.5, 20).unwrap();
                for _ in 0..7 {
                    border.tick().unwrap();
                }
            }
            let mut samples: [Vec<f64>; 5] = std::array::from_fn(|_| Vec::new());
            let names = [
                "border_two_corners",
                "border_one_extent",
                "status_lookup_corners",
                "status_lookup_extent",
                "status_resident_extent",
            ];
            let expected = chunks.iter().filter(|&&c| corners(&border, c)).count();
            assert_eq!(
                expected,
                chunks.iter().filter(|&&c| extent(&border, c)).count()
            );
            for repeat in 0..9 {
                // Rotate execution order to reduce systematic thermal/order bias.
                for k in 0..5 {
                    let variant = (k + repeat) % 5;
                    let b = black_box(&border);
                    let cs = black_box(&chunks);
                    let rs = black_box(&residents);
                    let ls = black_box(&levels);
                    let elapsed = measure(
                        || {
                            cs.iter()
                                .filter(|&&c| match variant {
                                    0 => corners(b, c),
                                    1 => extent(b, c),
                                    2 => {
                                        rs.contains_key(&c)
                                            && ls.loaded_entity_ticking(c.0, c.1)
                                            && corners(b, c)
                                    }
                                    3 => {
                                        rs.contains_key(&c)
                                            && ls.loaded_entity_ticking(c.0, c.1)
                                            && extent(b, c)
                                    }
                                    _ => ls.loaded_entity_ticking(c.0, c.1) && extent(b, c),
                                })
                                .count()
                        },
                        cs.len(),
                        rounds,
                        expected,
                    );
                    samples[variant].push(elapsed);
                }
            }
            for (i, values) in samples.iter_mut().enumerate() {
                values.sort_by(f64::total_cmp);
                println!(
                    "{scenario},{moving},{},{:.3},{:.3},{:.3},{expected}",
                    names[i], values[4], values[0], values[8]
                );
            }
        }
    }
}
