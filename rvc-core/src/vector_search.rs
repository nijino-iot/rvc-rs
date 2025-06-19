//! 向量搜索模块
//!
//! 实现类似 FAISS 的向量搜索功能，用于 RVC 中的检索增强语音转换

use crate::{RvcError, RvcResult};
use ndarray::{s, Array1, Array2, ArrayView1, ArrayView2};
use std::collections::HashMap;

/// 距离度量类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceMetric {
    /// 欧几里得距离
    L2,
    /// 内积距离（余弦相似度的变体）
    InnerProduct,
    /// 余弦距离
    Cosine,
}

/// 向量索引接口
pub trait VectorIndex: Send + Sync {
    /// 添加向量到索引
    fn add(&mut self, vectors: ArrayView2<f32>) -> RvcResult<()>;

    /// 搜索最近邻
    fn search(&self, queries: ArrayView2<f32>, k: usize) -> RvcResult<(Array2<f32>, Array2<i64>)>;

    /// 获取索引中的向量数量
    fn ntotal(&self) -> usize;

    /// 获取向量维度
    fn dimension(&self) -> usize;

    /// 重置索引
    fn reset(&mut self);
}

/// 平坦索引（暴力搜索）
pub struct FlatIndex {
    vectors: Array2<f32>,
    dimension: usize,
    metric: DistanceMetric,
}

impl FlatIndex {
    /// 创建新的平坦索引
    pub fn new(dimension: usize, metric: DistanceMetric) -> Self {
        Self {
            vectors: Array2::zeros((0, dimension)),
            dimension,
            metric,
        }
    }

    /// 计算距离
    fn compute_distances(&self, queries: ArrayView2<f32>) -> Array2<f32> {
        let n_queries = queries.nrows();
        let n_vectors = self.vectors.nrows();
        let mut distances = Array2::zeros((n_queries, n_vectors));

        match self.metric {
            DistanceMetric::L2 => {
                for i in 0..n_queries {
                    let query = queries.row(i);
                    for j in 0..n_vectors {
                        let vector = self.vectors.row(j);
                        let diff = &query - &vector;
                        distances[[i, j]] = diff.mapv(|x| x * x).sum().sqrt();
                    }
                }
            }
            DistanceMetric::InnerProduct => {
                for i in 0..n_queries {
                    let query = queries.row(i);
                    for j in 0..n_vectors {
                        let vector = self.vectors.row(j);
                        distances[[i, j]] = -query.dot(&vector); // 负内积（小值表示相似）
                    }
                }
            }
            DistanceMetric::Cosine => {
                for i in 0..n_queries {
                    let query = queries.row(i);
                    let query_norm = query.mapv(|x| x * x).sum().sqrt();
                    for j in 0..n_vectors {
                        let vector = self.vectors.row(j);
                        let vector_norm = vector.mapv(|x| x * x).sum().sqrt();
                        let dot_product = query.dot(&vector);

                        if query_norm > 1e-8 && vector_norm > 1e-8 {
                            distances[[i, j]] = 1.0 - dot_product / (query_norm * vector_norm);
                        } else {
                            distances[[i, j]] = 1.0; // 最大距离
                        }
                    }
                }
            }
        }

        distances
    }
}

impl VectorIndex for FlatIndex {
    fn add(&mut self, vectors: ArrayView2<f32>) -> RvcResult<()> {
        if vectors.ncols() != self.dimension {
            return Err(RvcError::vector_search(format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vectors.ncols()
            )));
        }

        if self.vectors.nrows() == 0 {
            self.vectors = vectors.to_owned();
        } else {
            // 拼接新向量
            let mut new_vectors =
                Array2::zeros((self.vectors.nrows() + vectors.nrows(), self.dimension));
            new_vectors
                .slice_mut(ndarray::s![..self.vectors.nrows(), ..])
                .assign(&self.vectors);
            new_vectors
                .slice_mut(ndarray::s![self.vectors.nrows().., ..])
                .assign(&vectors);
            self.vectors = new_vectors;
        }

        Ok(())
    }

    fn search(&self, queries: ArrayView2<f32>, k: usize) -> RvcResult<(Array2<f32>, Array2<i64>)> {
        if queries.ncols() != self.dimension {
            return Err(RvcError::vector_search(format!(
                "Query dimension mismatch: expected {}, got {}",
                self.dimension,
                queries.ncols()
            )));
        }

        if self.vectors.nrows() == 0 {
            return Err(RvcError::vector_search("Index is empty".to_string()));
        }

        let k = k.min(self.vectors.nrows());
        let n_queries = queries.nrows();

        let distances = self.compute_distances(queries);

        let mut result_distances = Array2::zeros((n_queries, k));
        let mut result_indices = Array2::zeros((n_queries, k));

        // 对每个查询找到最近的 k 个向量
        for i in 0..n_queries {
            let row_distances = distances.row(i);
            let mut indexed_distances: Vec<(f32, usize)> = row_distances
                .iter()
                .enumerate()
                .map(|(idx, &dist)| (dist, idx))
                .collect();

            // 排序：距离小的在前
            indexed_distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            for j in 0..k {
                result_distances[[i, j]] = indexed_distances[j].0;
                result_indices[[i, j]] = indexed_distances[j].1 as i64;
            }
        }

        Ok((result_distances, result_indices))
    }

    fn ntotal(&self) -> usize {
        self.vectors.nrows()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn reset(&mut self) {
        self.vectors = Array2::zeros((0, self.dimension));
    }
}

/// IVF (Inverted File) 索引
pub struct IVFIndex {
    centroids: Array2<f32>,
    clusters: HashMap<usize, Array2<f32>>,
    cluster_ids: HashMap<usize, Vec<i64>>, // 存储原始向量ID
    dimension: usize,
    metric: DistanceMetric,
    n_clusters: usize,
    next_id: i64,
}

impl IVFIndex {
    /// 创建新的 IVF 索引
    pub fn new(dimension: usize, n_clusters: usize, metric: DistanceMetric) -> Self {
        Self {
            centroids: Array2::zeros((0, dimension)),
            clusters: HashMap::new(),
            cluster_ids: HashMap::new(),
            dimension,
            metric,
            n_clusters,
            next_id: 0,
        }
    }

    /// 训练聚类中心
    pub fn train(&mut self, training_vectors: ArrayView2<f32>) -> RvcResult<()> {
        if training_vectors.ncols() != self.dimension {
            return Err(RvcError::vector_search(format!(
                "Training vector dimension mismatch: expected {}, got {}",
                self.dimension,
                training_vectors.ncols()
            )));
        }

        // 使用 K-means 算法训练聚类中心
        self.centroids = self.kmeans(training_vectors, self.n_clusters)?;

        Ok(())
    }

    /// K-means 聚类算法
    fn kmeans(&self, data: ArrayView2<f32>, k: usize) -> RvcResult<Array2<f32>> {
        let n_samples = data.nrows();
        let n_features = data.ncols();

        if n_samples < k {
            return Err(RvcError::vector_search(format!(
                "Not enough samples for {} clusters: got {}",
                k, n_samples
            )));
        }

        // 随机初始化聚类中心
        let mut centroids = Array2::zeros((k, n_features));
        for i in 0..k {
            let idx = (i * n_samples / k) % n_samples;
            centroids.row_mut(i).assign(&data.row(idx));
        }

        let max_iters = 100;
        let tolerance = 1e-4;

        for _iter in 0..max_iters {
            let mut new_centroids = Array2::zeros((k, n_features));
            let mut cluster_counts = vec![0; k];

            // 分配每个点到最近的聚类中心
            for i in 0..n_samples {
                let point = data.row(i);
                let mut min_dist = f32::INFINITY;
                let mut closest_cluster = 0;

                for j in 0..k {
                    let centroid = centroids.row(j);
                    let dist = self.compute_distance_single(point, centroid);
                    if dist < min_dist {
                        min_dist = dist;
                        closest_cluster = j;
                    }
                }

                // 累加到新的聚类中心
                for d in 0..n_features {
                    new_centroids[[closest_cluster, d]] += point[d];
                }
                cluster_counts[closest_cluster] += 1;
            }

            // 计算新的聚类中心
            for i in 0..k {
                if cluster_counts[i] > 0 {
                    for d in 0..n_features {
                        new_centroids[[i, d]] /= cluster_counts[i] as f32;
                    }
                }
            }

            // 检查收敛
            let mut max_change = 0.0_f64;
            for i in 0..k {
                for d in 0..n_features {
                    let change: f32 = (new_centroids[[i, d]] - centroids[[i, d]]).abs();
                    let change = change as f64;
                    if change > max_change {
                        max_change = change;
                    }
                }
            }

            centroids = new_centroids;

            if max_change < tolerance {
                break;
            }
        }

        Ok(centroids)
    }

    /// 计算单个向量之间的距离
    fn compute_distance_single(&self, a: ArrayView1<f32>, b: ArrayView1<f32>) -> f32 {
        match self.metric {
            DistanceMetric::L2 => {
                let diff = &a.to_owned() - &b.to_owned();
                diff.mapv(|x| x * x).sum().sqrt()
            }
            DistanceMetric::InnerProduct => -a.dot(&b),
            DistanceMetric::Cosine => {
                let a_norm = a.mapv(|x| x * x).sum().sqrt();
                let b_norm = b.mapv(|x| x * x).sum().sqrt();
                if a_norm > 1e-8 && b_norm > 1e-8 {
                    1.0 - a.dot(&b) / (a_norm * b_norm)
                } else {
                    1.0
                }
            }
        }
    }

    /// 找到向量最近的聚类中心
    fn find_closest_cluster(&self, vector: ArrayView1<f32>) -> usize {
        let mut min_dist = f32::INFINITY;
        let mut closest_cluster = 0;

        for i in 0..self.centroids.nrows() {
            let centroid = self.centroids.row(i);
            let dist = self.compute_distance_single(vector, centroid);
            if dist < min_dist {
                min_dist = dist;
                closest_cluster = i;
            }
        }

        closest_cluster
    }
}

impl VectorIndex for IVFIndex {
    fn add(&mut self, vectors: ArrayView2<f32>) -> RvcResult<()> {
        if vectors.ncols() != self.dimension {
            return Err(RvcError::vector_search(format!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vectors.ncols()
            )));
        }

        if self.centroids.nrows() == 0 {
            return Err(RvcError::vector_search(
                "Index not trained. Call train() first.".to_string(),
            ));
        }

        // 将每个向量分配到最近的聚类
        for i in 0..vectors.nrows() {
            let vector = vectors.row(i);
            let cluster_id = self.find_closest_cluster(vector);

            // 添加到对应的聚类
            if let Some(cluster_vectors) = self.clusters.get_mut(&cluster_id) {
                // 扩展现有聚类
                let new_size = cluster_vectors.nrows() + 1;
                let mut new_cluster = Array2::zeros((new_size, self.dimension));
                new_cluster
                    .slice_mut(s![..cluster_vectors.nrows(), ..])
                    .assign(cluster_vectors);
                new_cluster.row_mut(cluster_vectors.nrows()).assign(&vector);
                self.clusters.insert(cluster_id, new_cluster);
            } else {
                // 创建新聚类
                let mut new_cluster = Array2::zeros((1, self.dimension));
                new_cluster.row_mut(0).assign(&vector);
                self.clusters.insert(cluster_id, new_cluster);
                self.cluster_ids.insert(cluster_id, Vec::new());
            }

            // 记录原始ID
            self.cluster_ids
                .get_mut(&cluster_id)
                .unwrap()
                .push(self.next_id);
            self.next_id += 1;
        }

        Ok(())
    }

    fn search(&self, queries: ArrayView2<f32>, k: usize) -> RvcResult<(Array2<f32>, Array2<i64>)> {
        if queries.ncols() != self.dimension {
            return Err(RvcError::vector_search(format!(
                "Query dimension mismatch: expected {}, got {}",
                self.dimension,
                queries.ncols()
            )));
        }

        if self.centroids.nrows() == 0 {
            return Err(RvcError::vector_search(
                "Index not trained. Call train() first.".to_string(),
            ));
        }

        let n_queries = queries.nrows();
        let mut result_distances = Array2::zeros((n_queries, k));
        let mut result_indices = Array2::zeros((n_queries, k));

        // 对每个查询进行搜索
        for q in 0..n_queries {
            let query = queries.row(q);

            // 收集所有候选向量
            let mut candidates = Vec::new();

            // 简化：搜索所有聚类（实际应用中可以只搜索最近的几个聚类）
            for (cluster_id, cluster_vectors) in &self.clusters {
                let cluster_ids = &self.cluster_ids[cluster_id];

                for i in 0..cluster_vectors.nrows() {
                    let vector = cluster_vectors.row(i);
                    let dist = self.compute_distance_single(query, vector);
                    candidates.push((dist, cluster_ids[i]));
                }
            }

            // 排序并取前 k 个
            candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let k_actual = k.min(candidates.len());

            for i in 0..k_actual {
                result_distances[[q, i]] = candidates[i].0;
                result_indices[[q, i]] = candidates[i].1;
            }

            // 填充剩余位置
            for i in k_actual..k {
                result_distances[[q, i]] = f32::INFINITY;
                result_indices[[q, i]] = -1;
            }
        }

        Ok((result_distances, result_indices))
    }

    fn ntotal(&self) -> usize {
        self.clusters.values().map(|v| v.nrows()).sum()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn reset(&mut self) {
        self.centroids = Array2::zeros((0, self.dimension));
        self.clusters.clear();
        self.cluster_ids.clear();
        self.next_id = 0;
    }
}

/// 向量搜索工厂
pub struct VectorSearchFactory;

impl VectorSearchFactory {
    /// 创建平坦索引
    pub fn create_flat_index(dimension: usize, metric: DistanceMetric) -> Box<dyn VectorIndex> {
        Box::new(FlatIndex::new(dimension, metric))
    }

    /// 创建 IVF 索引
    pub fn create_ivf_index(
        dimension: usize,
        n_clusters: usize,
        metric: DistanceMetric,
    ) -> Box<dyn VectorIndex> {
        Box::new(IVFIndex::new(dimension, n_clusters, metric))
    }
}

/// 特征匹配器，用于 RVC 中的音色匹配
pub struct FeatureMatcher {
    index: Box<dyn VectorIndex>,
    features: Vec<Array1<f32>>,
}

impl FeatureMatcher {
    /// 创建新的特征匹配器
    pub fn new(index: Box<dyn VectorIndex>) -> Self {
        Self {
            index,
            features: Vec::new(),
        }
    }

    /// 添加特征向量
    pub fn add_features(&mut self, features: &[Array1<f32>]) -> RvcResult<()> {
        if features.is_empty() {
            return Ok(());
        }

        let dimension = features[0].len();
        let mut feature_matrix = Array2::zeros((features.len(), dimension));

        for (i, feature) in features.iter().enumerate() {
            if feature.len() != dimension {
                return Err(RvcError::vector_search(format!(
                    "Feature dimension mismatch at index {}: expected {}, got {}",
                    i,
                    dimension,
                    feature.len()
                )));
            }
            feature_matrix.row_mut(i).assign(feature);
        }

        self.index.add(feature_matrix.view())?;
        self.features.extend_from_slice(features);

        Ok(())
    }

    /// 查找最相似的特征
    pub fn find_similar(
        &self,
        query_features: &[Array1<f32>],
        k: usize,
    ) -> RvcResult<Vec<Vec<(f32, usize)>>> {
        if query_features.is_empty() {
            return Ok(Vec::new());
        }

        let dimension = query_features[0].len();
        let mut query_matrix = Array2::zeros((query_features.len(), dimension));

        for (i, feature) in query_features.iter().enumerate() {
            if feature.len() != dimension {
                return Err(RvcError::vector_search(format!(
                    "Query feature dimension mismatch at index {}: expected {}, got {}",
                    i,
                    dimension,
                    feature.len()
                )));
            }
            query_matrix.row_mut(i).assign(feature);
        }

        let (distances, indices) = self.index.search(query_matrix.view(), k)?;

        let mut results = Vec::new();
        for i in 0..query_features.len() {
            let mut query_results = Vec::new();
            for j in 0..k {
                let distance = distances[[i, j]];
                let index = indices[[i, j]];
                if index >= 0 {
                    query_results.push((distance, index as usize));
                }
            }
            results.push(query_results);
        }

        Ok(results)
    }

    /// 获取特征数量
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array;

    #[test]
    fn test_flat_index() -> RvcResult<()> {
        let mut index = FlatIndex::new(4, DistanceMetric::L2);

        // 添加一些向量
        let vectors = Array::from_shape_vec(
            (3, 4),
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        )
        .unwrap();

        index.add(vectors.view())?;
        assert_eq!(index.ntotal(), 3);

        // 搜索
        let queries = Array::from_shape_vec((1, 4), vec![1.0, 0.1, 0.0, 0.0]).unwrap();
        let (distances, indices) = index.search(queries.view(), 2)?;

        assert_eq!(distances.shape(), &[1, 2]);
        assert_eq!(indices.shape(), &[1, 2]);
        assert_eq!(indices[[0, 0]], 0); // 最近的应该是第一个向量

        Ok(())
    }

    #[test]
    fn test_ivf_index() -> RvcResult<()> {
        let mut index = IVFIndex::new(2, 2, DistanceMetric::L2);

        // 训练数据
        let training_data =
            Array::from_shape_vec((4, 2), vec![1.0, 1.0, 1.1, 0.9, -1.0, -1.0, -0.9, -1.1])
                .unwrap();

        index.train(training_data.view())?;

        // 添加向量
        let vectors = Array::from_shape_vec((2, 2), vec![1.0, 1.0, -1.0, -1.0]).unwrap();

        index.add(vectors.view())?;
        assert_eq!(index.ntotal(), 2);

        // 搜索
        let queries = Array::from_shape_vec((1, 2), vec![0.9, 0.9]).unwrap();
        let (distances, indices) = index.search(queries.view(), 1)?;

        assert_eq!(distances.shape(), &[1, 1]);
        assert_eq!(indices.shape(), &[1, 1]);

        Ok(())
    }

    #[test]
    fn test_feature_matcher() -> RvcResult<()> {
        let flat_index = VectorSearchFactory::create_flat_index(3, DistanceMetric::L2);
        let mut matcher = FeatureMatcher::new(flat_index);

        // 添加特征
        let features = vec![
            Array1::from(vec![1.0, 0.0, 0.0]),
            Array1::from(vec![0.0, 1.0, 0.0]),
            Array1::from(vec![0.0, 0.0, 1.0]),
        ];

        matcher.add_features(&features)?;
        assert_eq!(matcher.len(), 3);

        // 查找相似特征
        let query_features = vec![Array1::from(vec![0.9, 0.1, 0.0])];
        let results = matcher.find_similar(&query_features, 2)?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].len(), 2);
        assert_eq!(results[0][0].1, 0); // 最相似的应该是第一个特征

        Ok(())
    }

    #[test]
    fn test_distance_metrics() -> RvcResult<()> {
        // 测试不同的距离度量
        let vectors = Array::from_shape_vec((2, 3), vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0]).unwrap();

        let queries = Array::from_shape_vec((1, 3), vec![1.0, 0.0, 0.0]).unwrap();

        // L2 距离
        let mut index_l2 = FlatIndex::new(3, DistanceMetric::L2);
        index_l2.add(vectors.view())?;
        let (dist_l2, _) = index_l2.search(queries.view(), 2)?;

        // 内积距离
        let mut index_ip = FlatIndex::new(3, DistanceMetric::InnerProduct);
        index_ip.add(vectors.view())?;
        let (dist_ip, _) = index_ip.search(queries.view(), 2)?;

        // 余弦距离
        let mut index_cos = FlatIndex::new(3, DistanceMetric::Cosine);
        index_cos.add(vectors.view())?;
        let (dist_cos, _) = index_cos.search(queries.view(), 2)?;

        // L2: 查询向量与第一个向量完全相同，距离应该是0
        assert!((dist_l2[[0, 0]] - 0.0).abs() < 1e-6);

        // 内积: 查询向量与第一个向量的内积是1，所以距离是-1
        assert!((dist_ip[[0, 0]] - (-1.0)).abs() < 1e-6);

        // 余弦: 查询向量与第一个向量完全相同，距离应该是0
        assert!((dist_cos[[0, 0]] - 0.0).abs() < 1e-6);

        Ok(())
    }
}
