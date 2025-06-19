//! Tensor module using real PyTorch bindings (tch)
//!
//! This module provides a wrapper around tch::Tensor to maintain compatibility
//! with the existing RVC codebase while using real PyTorch functionality.

use crate::{RvcError, RvcResult};
use std::convert::TryFrom;

// Re-export tch types with our naming convention
pub use tch::{Device, Kind, Tensor as TchTensor};

/// Wrapper struct for tch::Tensor to provide additional functionality
#[derive(Debug)]
pub struct Tensor {
    inner: TchTensor,
}

impl Tensor {
    /// Create a new tensor from data slice
    pub fn from_slice(data: &[f32]) -> Self {
        let tensor = TchTensor::from_slice(data);
        Self { inner: tensor }
    }

    /// Create a tensor filled with zeros
    pub fn zeros(shape: &[i64], options: (Kind, Device)) -> Self {
        let tensor = TchTensor::zeros(shape, options);
        Self { inner: tensor }
    }

    /// Create a tensor filled with ones
    pub fn ones(shape: &[i64], options: (Kind, Device)) -> Self {
        let tensor = TchTensor::ones(shape, options);
        Self { inner: tensor }
    }

    /// Create a tensor with random normal distribution
    pub fn randn(shape: &[i64], options: (Kind, Device)) -> Self {
        let tensor = TchTensor::randn(shape, options);
        Self { inner: tensor }
    }

    /// Create a 1-D tensor with values from 0 to end
    pub fn arange(end: i64, options: (Kind, Device)) -> Self {
        let tensor = TchTensor::arange(end, options);
        Self { inner: tensor }
    }

    /// Create a 1-D tensor with values from start to end
    pub fn arange_start(start: i64, end: i64, options: (Kind, Device)) -> Self {
        let tensor = TchTensor::arange_start(start, end, options);
        Self { inner: tensor }
    }

    /// Get tensor shape
    pub fn size(&self) -> Vec<i64> {
        self.inner.size()
    }

    /// Get tensor shape as 3-tuple (for compatibility)
    pub fn size3(&self) -> RvcResult<(i64, i64, i64)> {
        let shape = self.size();
        if shape.len() != 3 {
            return Err(RvcError::TensorError(format!(
                "Expected 3D tensor, got {}D",
                shape.len()
            )));
        }
        Ok((shape[0], shape[1], shape[2]))
    }

    /// Get tensor device
    pub fn device(&self) -> Device {
        self.inner.device()
    }

    /// Get tensor kind (dtype)
    pub fn kind(&self) -> Kind {
        self.inner.kind()
    }

    /// Get both kind and device
    pub fn kind_device(&self) -> (Kind, Device) {
        (self.kind(), self.device())
    }

    /// Move tensor to specified device
    pub fn to_device(&self, device: Device) -> Self {
        Self {
            inner: self.inner.to_device(device),
        }
    }

    /// Create a shallow clone
    pub fn shallow_clone(&self) -> Self {
        Self {
            inner: self.inner.shallow_clone(),
        }
    }

    /// Create a deep copy
    pub fn copy(&self) -> Self {
        Self {
            inner: self.inner.copy(),
        }
    }

    /// Element-wise addition
    pub fn add(&self, other: &Tensor) -> Self {
        Self {
            inner: &self.inner + &other.inner,
        }
    }

    /// Element-wise subtraction
    pub fn sub(&self, other: &Tensor) -> Self {
        Self {
            inner: &self.inner - &other.inner,
        }
    }

    /// Element-wise multiplication
    pub fn mul(&self, other: &Tensor) -> Self {
        Self {
            inner: &self.inner * &other.inner,
        }
    }

    /// Scalar multiplication
    pub fn mul_scalar(&self, scalar: f64) -> Self {
        Self {
            inner: &self.inner * scalar,
        }
    }

    /// Element-wise division
    pub fn div(&self, other: &Tensor) -> Self {
        Self {
            inner: &self.inner / &other.inner,
        }
    }

    /// Square root
    pub fn sqrt(&self) -> Self {
        Self {
            inner: self.inner.sqrt(),
        }
    }

    /// Sine function
    pub fn sin(&self) -> Self {
        Self {
            inner: self.inner.sin(),
        }
    }

    /// Cosine function
    pub fn cos(&self) -> Self {
        Self {
            inner: self.inner.cos(),
        }
    }

    /// Power function (tensor^scalar)
    pub fn pow_tensor_scalar(&self, exponent: f64) -> Self {
        Self {
            inner: self.inner.pow_tensor_scalar(exponent),
        }
    }

    /// Absolute value
    pub fn abs(&self) -> Self {
        Self {
            inner: self.inner.abs(),
        }
    }

    /// Complex angle (for complex tensors)
    pub fn angle(&self) -> Self {
        Self {
            inner: self.inner.angle(),
        }
    }

    /// Floor function
    pub fn floor(&self) -> Self {
        Self {
            inner: self.inner.floor(),
        }
    }

    /// Matrix multiplication
    pub fn matmul(&self, other: &Tensor) -> Self {
        Self {
            inner: self.inner.matmul(&other.inner),
        }
    }

    /// Transpose tensor
    pub fn transpose(&self, dim0: i64, dim1: i64) -> Self {
        Self {
            inner: self.inner.transpose(dim0, dim1),
        }
    }

    /// Add singleton dimension
    pub fn unsqueeze(&self, dim: i64) -> Self {
        Self {
            inner: self.inner.unsqueeze(dim),
        }
    }

    /// Expand tensor to new shape
    pub fn expand(&self, shape: &[i64], implicit: bool) -> Self {
        Self {
            inner: self.inner.expand(shape, implicit),
        }
    }

    /// Reshape tensor
    pub fn view(&self, shape: &[i64]) -> Self {
        Self {
            inner: self.inner.view(shape),
        }
    }

    /// Reshape tensor (alias for view)
    pub fn reshape(&self, shape: &[i64]) -> Self {
        self.view(shape)
    }

    /// Convert tensor to specified kind (dtype)
    pub fn to_kind(&self, kind: Kind) -> Self {
        Self {
            inner: self.inner.to_kind(kind),
        }
    }

    /// In-place copy from another tensor
    pub fn copy_(&mut self, src: &Tensor) -> &mut Self {
        self.inner.copy_(&src.inner);
        self
    }

    /// Natural logarithm
    pub fn log(&self) -> Self {
        Self {
            inner: self.inner.log(),
        }
    }

    /// Round to nearest integer
    pub fn round(&self) -> Self {
        Self {
            inner: self.inner.round(),
        }
    }

    /// Narrow tensor along dimension
    pub fn narrow(&self, dim: i64, start: i64, length: i64) -> Self {
        Self {
            inner: self.inner.narrow(dim, start, length),
        }
    }

    /// Slice scatter operation (compatibility wrapper)
    pub fn slice_scatter(&self, src: &Tensor, dim: i64, start: i64, step: i64) -> Self {
        // For the RVC use case, we need to handle setting values at specific positions
        // This is a simplified implementation that handles the common pattern
        let mut result = self.copy();
        let src_size = src.size();
        let self_size = self.size();

        if dim == 0 && step == 1 && start >= 0 && start < self_size[0] {
            // Handle 1D case: result[start:start+src_len] = src
            let end = (start + src_size[0]).min(self_size[0]);
            let actual_length = end - start;

            if actual_length > 0 {
                let narrow_src = if src_size[0] > actual_length {
                    src.narrow(0, 0, actual_length)
                } else {
                    src.copy()
                };

                // Use tch's slice_scatter with proper parameters
                result = Self {
                    inner: result.inner.slice_scatter(
                        &narrow_src.inner,
                        dim,
                        Some(start),
                        Some(end),
                        1,
                    ),
                };
            }
        }
        result
    }

    /// Index into tensor
    pub fn get(&self, index: i64) -> Self {
        Self {
            inner: self.inner.get(index),
        }
    }

    /// Real FFT (compatibility wrapper)
    pub fn fft_rfft(&self, dims: &[i64], normalized: bool) -> Self {
        // Convert old API to new tch API
        let dim = if dims.is_empty() { -1 } else { dims[0] };
        let norm = if normalized { Some("ortho") } else { None };
        Self {
            inner: self.inner.fft_rfft(None, dim, norm.unwrap_or("backward")),
        }
    }

    /// Softmax function
    pub fn softmax(&self, dim: i64, _kind: Kind) -> Self {
        Self {
            inner: self.inner.softmax(dim, self.kind()),
        }
    }

    /// ReLU activation
    pub fn relu(&self) -> Self {
        Self {
            inner: self.inner.relu(),
        }
    }

    /// Dropout (training mode)
    pub fn dropout(&self, p: f64, train: bool) -> Self {
        Self {
            inner: self.inner.dropout(p, train),
        }
    }

    /// Make tensor contiguous
    pub fn contiguous(&self) -> Self {
        Self {
            inner: self.inner.contiguous(),
        }
    }

    /// Sum along dimensions
    pub fn sum_dim_intlist(&self, dims: &[i64], keep_dim: bool, dtype: Option<Kind>) -> Self {
        Self {
            inner: self.inner.sum_dim_intlist(dims, keep_dim, dtype),
        }
    }

    /// Sum along single dimension (compatibility wrapper)
    pub fn sum_dim(&self, dim: i64, keep_dim: bool, dtype: Kind) -> Self {
        Self {
            inner: self
                .inner
                .sum_dim_intlist(&[dim][..], keep_dim, Some(dtype)),
        }
    }

    /// Concatenate tensors
    pub fn cat(tensors: &[&Tensor], dim: i64) -> Self {
        let tch_tensors: Vec<&TchTensor> = tensors.iter().map(|t| &t.inner).collect();
        Self {
            inner: TchTensor::cat(&tch_tensors, dim),
        }
    }

    /// Stack tensors
    pub fn stack(tensors: &[&Tensor], dim: i64) -> Self {
        let tch_tensors: Vec<&TchTensor> = tensors.iter().map(|t| &t.inner).collect();
        Self {
            inner: TchTensor::stack(&tch_tensors, dim),
        }
    }

    /// Create tensor with same shape filled with zeros
    pub fn zeros_like(&self) -> Self {
        Self {
            inner: self.inner.zeros_like(),
        }
    }

    /// Fill tensor with value
    pub fn fill(&self, value: f64) -> Self {
        Self {
            inner: self.inner.fill(value),
        }
    }

    /// Convert from tch::Tensor
    pub fn from(tensor: TchTensor) -> Self {
        Self { inner: tensor }
    }

    /// Create tensor from scalar value
    pub fn scalar(value: f64) -> Self {
        Self {
            inner: TchTensor::from(value),
        }
    }

    /// Get inner tch::Tensor
    pub fn inner(&self) -> &TchTensor {
        &self.inner
    }

    /// Convert to inner tch::Tensor
    pub fn into_inner(self) -> TchTensor {
        self.inner
    }
}

impl Clone for Tensor {
    fn clone(&self) -> Self {
        self.copy()
    }
}

// Implement arithmetic operators
impl std::ops::Add for Tensor {
    type Output = Tensor;
    fn add(self, other: Tensor) -> Tensor {
        Self {
            inner: &self.inner + &other.inner,
        }
    }
}

impl std::ops::Add for &Tensor {
    type Output = Tensor;
    fn add(self, other: &Tensor) -> Tensor {
        self.add(other)
    }
}

impl std::ops::Sub for Tensor {
    type Output = Tensor;
    fn sub(self, other: Tensor) -> Tensor {
        Self {
            inner: &self.inner - &other.inner,
        }
    }
}

impl std::ops::Sub for &Tensor {
    type Output = Tensor;
    fn sub(self, other: &Tensor) -> Tensor {
        self.sub(other)
    }
}

impl std::ops::Mul for Tensor {
    type Output = Tensor;
    fn mul(self, other: Tensor) -> Tensor {
        Self {
            inner: &self.inner * &other.inner,
        }
    }
}

impl std::ops::Mul for &Tensor {
    type Output = Tensor;
    fn mul(self, other: &Tensor) -> Tensor {
        self.mul(other)
    }
}

impl std::ops::Mul<f64> for Tensor {
    type Output = Tensor;
    fn mul(self, scalar: f64) -> Tensor {
        self.mul_scalar(scalar)
    }
}

impl std::ops::Mul<f64> for &Tensor {
    type Output = Tensor;
    fn mul(self, scalar: f64) -> Tensor {
        self.mul_scalar(scalar)
    }
}

impl std::ops::Add<f64> for Tensor {
    type Output = Tensor;
    fn add(self, scalar: f64) -> Tensor {
        Tensor {
            inner: &self.inner + scalar,
        }
    }
}

impl std::ops::Add<f64> for &Tensor {
    type Output = Tensor;
    fn add(self, scalar: f64) -> Tensor {
        Tensor {
            inner: &self.inner + scalar,
        }
    }
}

impl std::ops::Div<f64> for Tensor {
    type Output = Tensor;
    fn div(self, scalar: f64) -> Tensor {
        Tensor {
            inner: &self.inner / scalar,
        }
    }
}

impl std::ops::Div for &Tensor {
    type Output = Tensor;
    fn div(self, other: &Tensor) -> Tensor {
        self.div(other)
    }
}

// Conversion to Vec<f32>
impl TryFrom<Tensor> for Vec<f32> {
    type Error = RvcError;
    fn try_from(tensor: Tensor) -> Result<Vec<f32>, Self::Error> {
        match Vec::<f32>::try_from(tensor.inner) {
            Ok(vec) => Ok(vec),
            Err(e) => Err(RvcError::TensorError(format!(
                "Failed to convert tensor to Vec<f32>: {}",
                e
            ))),
        }
    }
}

/// CUDA utilities
pub struct Cuda;

impl Cuda {
    /// Check if CUDA is available
    pub fn is_available() -> bool {
        tch::Cuda::is_available()
    }

    /// Get number of CUDA devices
    pub fn device_count() -> i64 {
        tch::Cuda::device_count()
    }
}

/// Execute closure without gradient computation
pub fn no_grad<T, F>(f: F) -> T
where
    F: FnOnce() -> T,
{
    tch::no_grad(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_creation() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        let tensor = Tensor::from_slice(&data);
        assert_eq!(tensor.size(), vec![4]);
    }

    #[test]
    fn test_tensor_operations() {
        let a = Tensor::from_slice(&[1.0, 2.0, 3.0]);
        let b = Tensor::from_slice(&[4.0, 5.0, 6.0]);
        let c = a.add(&b);

        // Basic operation test - exact values depend on tch implementation
        assert_eq!(c.size(), vec![3]);
    }

    #[test]
    fn test_cuda_availability() {
        // Just test that the function doesn't panic
        let _available = Cuda::is_available();
        let _count = Cuda::device_count();
    }
}
