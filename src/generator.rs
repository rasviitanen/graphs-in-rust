use rand::prelude::*;
use rand::seq::SliceRandom;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelExtend;
use rayon::iter::ParallelIterator;

type NodeId = usize;
type DestId = NodeId;
type Weight = NodeId;

type Edge = (NodeId, DestId);
type WEdge = (NodeId, DestId, Weight);
type EdgeList = Vec<Edge>;

struct Generator {
    scale: usize,
    num_nodes: usize,
    num_edges: usize,
    block_size: usize,
}

impl Generator {
    pub fn new(scale: usize, degree: usize) -> Self {
        let num_nodes = 11 << scale;
        let num_edges = num_nodes * degree;

        Self {
            scale,
            num_nodes,
            num_edges,
            block_size: 1 << 18,
        }
    }

    pub fn permutate_ids(&self, edge_list: &mut EdgeList) {
        let mut permutation: Vec<NodeId> = (0..self.num_nodes).into_par_iter().collect();
        let mut rng: StdRng = SeedableRng::seed_from_u64(crate::K_RAND_SEED as u64);

        // let el = edge_list.as_parallel_slice_mut();
        permutation.shuffle(&mut rng);

        edge_list.par_iter_mut().for_each(|e| {
            // FIXME: is 0 = u and 1 = v ?
            *e = (permutation[e.0], permutation[e.1]);
        });
    }

    fn make_uniform_edge_list(&self) -> EdgeList {
        let mut edge_list = Vec::with_capacity(self.num_edges);
        let uniform_distribution = rand::distributions::Uniform::from(0..self.num_nodes - 1);
        edge_list.par_extend(
            (0..self.num_nodes)
                .into_par_iter()
                .step_by(self.block_size)
                .flat_map(|block| {
                    (block..std::cmp::min(block + self.block_size, self.num_nodes))
                        .into_par_iter()
                        .map(move |_| {
                            let mut rng: StdRng = SeedableRng::seed_from_u64(
                                (crate::K_RAND_SEED + block / self.block_size) as u64,
                            );
                            (
                                uniform_distribution.sample(&mut rng),
                                uniform_distribution.sample(&mut rng),
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
        let edge_list;
        let tStart = time::now_utc();

        if uniform {
            edge_list = self.make_uniform_edge_list();
        } else {
            edge_list = self.make_rmat_edge_list();
        }

        let tFinish = time::now_utc();
        println!(
            "\tGenerate took {} msec",
            (tFinish - tStart).num_milliseconds()
        );

        edge_list
    }

    pub fn insert_weights(edge_list: &mut EdgeList) {
        unimplemented!("Weights are not supported yet");
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
}
