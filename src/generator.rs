use crate::types::*;

use rand::prelude::*;
use rand::seq::SliceRandom;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;
use std::fs::File;
use std::io::{prelude::*, BufReader};

pub struct Generator {
    scale: usize,
    num_nodes: usize,
    num_edges: usize,
    block_size: usize,
}

impl Generator {
    pub fn new(scale: usize, degree: usize) -> Self {
        let num_nodes = 1 << scale;
        let num_edges = num_nodes * degree;
        dbg!(num_nodes);
        dbg!(num_edges);
        dbg!(degree);

        Self {
            scale,
            num_nodes,
            num_edges,
            block_size: 1 << 18,
        }
    }

    pub fn permutate_ids(&self, edge_list: &mut EdgeList) {
        let mut permutation: Vec<NodeId> = (0..self.num_nodes).into_par_iter().collect();
        // let mut rng: StdRng = SeedableRng::seed_from_u64(crate::K_RAND_SEED as u64);

        // FIXME: Change to custom seed?
        let mut rng = rand::thread_rng();

        permutation.shuffle(&mut rng);

        edge_list.par_iter_mut().for_each(|e| {
            // FIXME: is 0 = u and 1 = v ?
            *e = (permutation[e.0], permutation[e.1], None);
        });
    }

    fn make_uniform_edge_list(&self) -> EdgeList {
        let mut edge_list = Vec::with_capacity(self.num_edges);
        let uniform_distribution = rand::distributions::Uniform::from(0..self.num_nodes);
        edge_list.par_extend(
            (0..self.num_edges)
                .into_par_iter()
                .step_by(self.block_size)
                .flat_map(|block| {
                    (block..std::cmp::min(block + self.block_size, self.num_edges))
                        .into_par_iter()
                        .map(move |_| {
                            // let mut rng = SeedableRng::seed_from_u64(
                            //     (crate::K_RAND_SEED + block / self.block_size) as u64,
                            // );

                            // FIXME: change to custom seed?
                            let mut rng = rand::thread_rng();
                            (
                                uniform_distribution.sample(&mut rng),
                                uniform_distribution.sample(&mut rng),
                                None,
                            )
                        })
                }),
        );

        edge_list
    }

    fn make_rmat_edge_list(&self) -> EdgeList {
        unimplemented!("RMAT edge generation is not implemented yet");
    }

    pub fn generate_edge_list(&self, uniform: bool) -> EdgeList {
        let t_start = time::now_utc();

        let edge_list = if uniform {
            self.make_uniform_edge_list()
        } else {
            self.make_rmat_edge_list()
        };

        let t_finish = time::now_utc();
        println!(
            "\tGenerate took {} msec",
            (t_finish - t_start).num_milliseconds()
        );

        edge_list
    }

    pub fn generate_edge_list_from_file(&self, file: &str) -> EdgeList {
        let mut edge_list = Vec::new();

        let file = File::open(file).unwrap();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.unwrap();
            let line_parts: Vec<_> = line.split(|c| c == ' ' || c == '\t').collect();

            let connection = (
                line_parts[0].parse::<usize>().unwrap(),
                line_parts[1].parse::<usize>().unwrap(),
                line_parts.get(2).map(|x| x.parse::<usize>().unwrap()),
            );

            edge_list.push(connection);
        }

        edge_list
    }

    pub fn insert_weights(edge_list: &mut EdgeList) {
        let uniform_distribution = rand::distributions::Uniform::from(1..256);
        let mut rng = rand::thread_rng();

        let el_len = edge_list.len();

        for e in edge_list.iter_mut() {
            *e = (e.0, e.1, Some(uniform_distribution.sample(&mut rng)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let generator = Generator::new(12, 4);
        let edge_list = generator.generate_edge_list(true);
        assert_eq!(edge_list.len(), 45056);
    }

    #[test]
    fn generate_small() {
        let generator = Generator::new(1, 1);
        let edge_list = generator.generate_edge_list(true);
        assert_eq!(edge_list.len(), 11 << 1);
    }
}
