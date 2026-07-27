use crate::linalg::matrix::Matrix;

/// Computes the Mean Squared Error (MSE) Loss between Predictions and True Targets
pub fn mse_loss<const N: usize, const OUT: usize>(
    predictions: &Matrix<N, OUT>,
    targets: &Matrix<N, OUT>,
) -> f32 {
    let diff = *predictions - *targets;
    let mut sum_sq = 0.0;
    
    for i in 0..N {
        for j in 0..OUT {
            sum_sq += diff.data[i][j] * diff.data[i][j];
        }
    }
    
    sum_sq / (N as f32)
}

/// Computes the Gradients and performs one step of Gradient Descent.
/// 
/// 數學微積分推導 (Calculus Derivative):
/// y_pred = X * W
/// Loss = MSE = (1/N) * sum((y_pred - y_true)^2)
/// dL/dW (Loss 對權重 W 的偏微分) = (2/N) * X^T * (y_pred - y_true)
///
/// X: [N x IN]
/// W: [IN x OUT]
/// y_pred, y_true: [N x OUT]
pub fn gradient_descent_step<const N: usize, const IN: usize, const OUT: usize>(
    inputs: &Matrix<N, IN>,
    weights: &mut Matrix<IN, OUT>,
    predictions: &Matrix<N, OUT>,
    targets: &Matrix<N, OUT>,
    learning_rate: f32,
) {
    let diff = *predictions - *targets;
    
    // X^T : [IN x N]
    let inputs_t = inputs.transpose();
    
    // dL/dW = X^T * (y_pred - y_true) : [IN x N] * [N x OUT] = [IN x OUT]
    let mut grad_w = inputs_t * diff;
    
    // Multiply by 2/N
    let factor = 2.0 / (N as f32);
    grad_w = grad_w * factor;
    
    // W_new = W_old - learning_rate * dL/dW
    let update = grad_w * learning_rate;
    *weights = *weights - update;
}
