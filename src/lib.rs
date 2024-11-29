//! # A Density-Based Algorithm for Discovering Clusters
//!
//! This algorithm finds all points within `eps` distance of each other and
//! attempts to cluster them. If there are at least `mpt` points reachable
//! (within distance `eps`) from a given point P, then all reachable points are
//! clustered together. The algorithm then attempts to expand the cluster,
//! finding all border points reachable from each point in the cluster
//!
//!
//! See `Ester, Martin, et al. "A density-based algorithm for discovering
//! clusters in large spatial databases with noise." Kdd. Vol. 96. No. 34.
//! 1996.` for the original paper
//!
//! Thanks to the rusty_machine implementation for inspiration
// extern crate blas_src;

use ndarray::*;
use ndarray_linalg::*;

use Classification::{Core, Edge, Noise};

/// Calculate euclidean distance between two vectors
///
/// This is the default distance function
#[inline]
pub fn euclidean_distance<T>(a: &[T], b: &[T]) -> f64
where
    f64: From<T>,
    T: Copy,
{
    a.iter()
        .zip(b.iter())
        .fold(0f64, |acc, (&x, &y)| {
            acc + (f64::from(x) - f64::from(y)).powi(2)
        })
        .sqrt()
}

/// Classification according to the DBSCAN algorithm
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum Classification {
    /// A point with at least `min_points` neighbors within `eps` diameter
    Core(usize),
    /// A point within `eps` of a core point, but has less than `min_points` neighbors
    Edge(usize),
    /// A point with no connections
    Noise,
}

/// Cluster datapoints using the DBSCAN algorithm
///
/// # Arguments
/// * `eps` - maximum distance between datapoints within a cluster
/// * `min_points` - minimum number of datapoints to make a cluster
/// * `input` - a Vec<Vec<f64>> of datapoints, organized by row
pub fn cluster<T>(eps: f64, min_points: usize, input: &Vec<Vec<T>>) -> Vec<Classification>
where
    T: Copy
        + std::ops::Sub<Output = T>
        + std::ops::SubAssign
        + ndarray::LinalgScalar
        + ndarray_linalg::Lapack,
    f64: From<T>,
{
    Model::new(eps, min_points, euclidean_distance).run(input)
}

/// DBSCAN parameters
pub struct Model<T>
where
    T: Copy + std::ops::Sub<Output = T> + std::ops::SubAssign + ndarray::LinalgScalar,
    f64: From<T>,
{
    /// Epsilon value - maximum distance between points in a cluster
    pub eps: f64,
    /// Minimum number of points in a cluster
    pub mpt: usize,

    distance: fn(&[T], &[T]) -> f64,
    c: Vec<Classification>,
    v: Vec<bool>,
}

impl<T> Model<T>
where
    T: Copy
        + std::ops::Sub<Output = T>
        + std::ops::SubAssign
        + ndarray::LinalgScalar
        + ndarray_linalg::Lapack,
    f64: From<T>,
{
    /// Create a new `Model` with a set of parameters
    ///
    /// # Arguments
    /// * `eps` - maximum distance between datapoints within a cluster
    /// * `min_points` - minimum number of datapoints to make a cluster
    pub fn new(eps: f64, min_points: usize, distance: fn(&[T], &[T]) -> f64) -> Model<T> {
        Model {
            eps,
            mpt: min_points,
            c: Vec::new(),
            v: Vec::new(),
            distance: distance,
        }
    }

    /// Change the function used to calculate distance between points.
    /// Euclidean distance is the default measurement used.
    pub fn set_distance_fn<F>(mut self, func: fn(&[T], &[T]) -> f64) -> Model<T> {
        self.distance = func;
        self
    }

    fn expand_cluster(
        &mut self,
        population: &[Vec<T>],
        queue: &mut Vec<usize>,
        cluster: usize,
    ) -> bool {
        let mut new_cluster = false;
        while let Some(ind) = queue.pop() {
            let neighbors = self.range_query(&population[ind], population);
            if neighbors.len() < self.mpt {
                continue;
            }
            new_cluster = true;
            self.c[ind] = Core(cluster);
            for n_idx in neighbors {
                // n_idx is at least an edge point
                if self.c[n_idx] == Noise {
                    self.c[n_idx] = Edge(cluster);
                }

                if self.v[n_idx] {
                    continue;
                }

                self.v[n_idx] = true;
                queue.push(n_idx);
            }
        }
        new_cluster
    }

    #[inline]
    fn range_query(&self, sample: &[T], population: &[Vec<T>]) -> Vec<usize> {
        let population = population.to_vec();
        let shape = (population.len(), population[0].len());
        let sample_repeated = Array2::from_shape_fn(shape, |(i, j)| sample[j]);
        let population: Array2<T> =
            Array2::from_shape_vec(shape, population.iter().flat_map(|x| x.to_vec()).collect())
                .unwrap();

        // norm_l2 between sample and population
        // diff
        let diff = &population - &sample_repeated;
        // l2 norm of every row using linalg.norm_l2
        let norms = diff.map_axis(Axis(1), |x| x.norm_l2());
        let idxs = norms
            .iter()
            .enumerate()
            .filter_map(|(id, &x)| {
                if self.eps > x {
                    return Some(id);
                }
                return None;
            })
            .collect();
        // filter

        return vec![];

        // population
        //     .iter()
        //     .enumerate()
        //     .filter(|(_, pt)| (self.distance)(sample, pt) < self.eps)
        //     .map(|(idx, _)| idx)
        //     .collect()
    }

    /// Run the DBSCAN algorithm on a given population of datapoints.
    ///
    /// A vector of [`Classification`] enums is returned, where each element
    /// corresponds to a row in the input matrix.
    ///
    /// # Arguments
    /// * `population` - a matrix of datapoints, organized by rows
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbscan::Classification::*;
    /// use dbscan::Model;
    ///
    /// let model = Model::new(1.0, 3);
    /// let inputs = vec![
    ///     vec![1.5, 2.2],
    ///     vec![1.0, 1.1],
    ///     vec![1.2, 1.4],
    ///     vec![0.8, 1.0],
    ///     vec![3.7, 4.0],
    ///     vec![3.9, 3.9],
    ///     vec![3.6, 4.1],
    ///     vec![10.0, 10.0],
    /// ];
    /// let output = model.run(&inputs);
    /// assert_eq!(
    ///     output,
    ///     vec![
    ///         Edge(0),
    ///         Core(0),
    ///         Core(0),
    ///         Core(0),
    ///         Core(1),
    ///         Core(1),
    ///         Core(1),
    ///         Noise
    ///     ]
    /// );
    /// ```
    pub fn run(mut self, population: &Vec<Vec<T>>) -> Vec<Classification> {
        self.c = vec![Noise; population.len()];
        self.v = vec![false; population.len()];

        let mut cluster = 0;
        let mut queue = Vec::new();

        for idx in 0..population.len() {
            if self.v[idx] {
                continue;
            }

            self.v[idx] = true;

            queue.push(idx);

            if self.expand_cluster(population, &mut queue, cluster) {
                cluster += 1;
            }
        }
        self.c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster() {
        let model = Model::new(1.0, 3, euclidean_distance);
        let inputs = vec![
            vec![1.5, 2.2],
            vec![1.0, 1.1],
            vec![1.2, 1.4],
            vec![0.8, 1.0],
            vec![3.7, 4.0],
            vec![3.9, 3.9],
            vec![3.6, 4.1],
            vec![10.0, 10.0],
        ];
        let output = model.run(&inputs);
        assert_eq!(
            output,
            vec![
                Edge(0),
                Core(0),
                Core(0),
                Core(0),
                Core(1),
                Core(1),
                Core(1),
                Noise
            ]
        );
    }

    #[test]
    fn cluster_edge() {
        let model = Model::new(0.253110, 3, euclidean_distance);
        let inputs = vec![
            vec![
                0.3311755015020835,
                0.20474852214361858,
                0.21050489388506638,
                0.23040992344219402,
                0.023161159027037505,
            ],
            vec![
                0.5112445458548497,
                0.1898442816540571,
                0.11674072294944157,
                0.14853288499259437,
                0.03363756454905728,
            ],
            vec![
                0.581134172697341,
                0.15084733646825743,
                0.09997992993087741,
                0.13580335513916678,
                0.03223520576435743,
            ],
            vec![
                0.17210416043100868,
                0.3403172702783598,
                0.18218098373740396,
                0.2616980943829193,
                0.04369949117030829,
            ],
        ];
        let output = model.run(&inputs);
        assert_eq!(output, vec![Core(0), Core(0), Edge(0), Edge(0)]);
    }

    #[test]
    fn range_query() {
        let model = Model::new(1.0, 3, euclidean_distance);
        let inputs = vec![vec![1.0, 1.0], vec![1.1, 1.9], vec![3.0, 3.0]];
        let neighbours = model.range_query(&[1.0, 1.0], &inputs);

        assert!(neighbours.len() == 2);
    }

    #[test]
    fn range_query_small_eps() {
        let model = Model::new(0.01, 3, euclidean_distance);
        let inputs = vec![vec![1.0, 1.0], vec![1.1, 1.9], vec![3.0, 3.0]];
        let neighbours = model.range_query(&[1.0, 1.0], &inputs);

        assert!(neighbours.len() == 1);
    }

    fn taxicab(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b.iter()).fold(0f64, |acc, (&x, &y)| {
            acc + (f64::from(x) - f64::from(y)).abs()
        })
    }

    #[test]
    fn range_query_custom_distance() {
        let model = Model::new(1.0, 3, euclidean_distance)
            .set_distance_fn::<fn(&[f64], &[f64]) -> f64>(taxicab);
        let inputs = vec![vec![1.0, 1.0], vec![1.1, 1.9], vec![3.0, 3.0]];
        let neighbours = model.range_query(&[1.0, 1.0], &inputs);
        assert_eq!(neighbours.len(), 1)
    }
}
